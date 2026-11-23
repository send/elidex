# Compute item display before publishing style

C1b.2-pre follows C1b.1 (#740). The Flex collector currently changes the item's display and reinserts its entire computed style. The shared child dispatcher reads that display when deciding whether an auto-width inline container needs shrink-to-fit. Move the computed-display decision to style resolution so both pre-layout readers and the first child dispatch see the same value.

## §1. Carry the effective ancestor through style resolution

Start the document walk with no effective ancestor. For each element carry the nearest flat-tree ancestor element's computed display, skipping contents. Pass this input separately from the immediate inheritance parent used by the cascade. A contents element retains its own computed style and passes the incoming display onward. Every other element starts a new context for its children, including none. A flex element inside a hidden subtree still affects its children. Thread the value by descent and restore it for siblings. Follow the composed host, slot, assigned-node and fallback paths in walk.rs. A document or shadow root introduces no element display.

Use one style finalization rule for Flex, InlineFlex, Grid and InlineGrid parents. Apply the existing Display::blockify conversion after the element's display has been resolved and before the first ECS insertion in resolve_and_attach_style. Preserve the absolute/fixed float reset and float blockification in resolve/mod.rs. Generated before/after entities use the same finalizer after pseudo.rs has supplied the default inline display, with the originating element as their parent and the same contents traversal. Anonymous text items receive no element style adjustment. Recompute on restyle from the cascade and the current ancestry. Layout receives the published display without a new per-item component or an ancestor lookup.

Flexbox §4 and Grid §6.1 define the ancestor predicate even for a parent that generates no container box. CSS Display §2.7 defines the conversion, including table-internal and box-suppressing values. [Flex items](https://www.w3.org/TR/css-flexbox-1/#flex-items) · [Grid item display](https://www.w3.org/TR/css-grid-1/#grid-item-display) · [Automatic box type transformations](https://www.w3.org/TR/css-display-3/#transformations)

## §2. Crossing checks

| Crossing | Production route | Required observation |
|---|---|---|
| Contents × auto-width inline-flex | Author stylesheet → style walk → ECS → dispatch_layout_child | Use a 200px column Flex with two contents wrappers and an auto-width inline-flex child containing a fixed 20px descendant. Before any layout the child computes to Flex, width Auto, and inherits the inner wrapper's distinct color. At the initial auto-width dispatch observe the Flex route, the incoming width and absence of the inline shrink-to-fit adjustment. Final stretch gives 200px. A normal block ancestor inserted between wrapper and item restores InlineFlex on restyle and the ordinary auto-width dispatch shrinks to 20px. Inspect the branch on dispatch before any placement override can mask it. |
| Siblings × inheritance | Style two branches, one wrapped by a block and one promoted through contents | Only the promoted inline item computes to Block. Flip the container between flex and block and back, and assert each sibling's display after each style pass before layout. |
| Parallel display routes | InlineGrid parent, Flex below a hidden ancestor, generated content and shadow traversal | A table-cell child under InlineGrid computes to Block. A hidden ancestor above a Flex parent still yields Flex for its inline-flex child. None and Contents children retain those values. An implicit-inline generated before under Flex computes to Block. Check a slotted inline element and fallback child with different effective formatting parents, verifying inheritance independently. |
| Collection × placement ownership | Style producer → collect_flex_items → relayout_item_at_position | In this prerequisite assert pre-layout display and the collector's display predicate is already satisfied. Record both existing writes for C1b.2. In that child remove the collection and placement writes together, observe zero computed-style updates on real probes and two placements, and compare the whole component including all four physical margins. |

## §3. Spec coverage map

| Spec section | Step | Branch | Touch (compile/dispatch site) | Full enum? | User-input flow |
|---|---|---|---|---|---|
| css-flexbox-1 §4 Flex Items | choose effective ancestor | flex, inline-flex, other, root | walk_tree, walk_shadow_child, resolve_and_attach_style | both Flex container values | yes |
| css-grid-1 §6.1 Grid Item Display | select same conversion | grid, inline-grid | style finalization before ECS publication | both Grid container values | yes |
| css-display-3 §2.5 Box Generation | carry ancestor display | contents, none, box | walk_children and generated pseudo style | both box-suppression values | yes |
| css-display-3 §2.7 Automatic Box Type Transformations | convert computed display | inline variants, table internals, unchanged values | Display::blockify → dispatch_layout_child | existing Display mapping | yes |

At the implementation head inventory element and pseudo style publishers, root entry points and shadow/slot recursion. Migrate layout-unit fixtures that construct ECS directly to use computed display when C1b.2 retires the Flex collector write. Keep the production pre-layout crossing in this prerequisite. Run the all-features tests for elidex-style, elidex-layout, elidex-layout-flex and elidex-layout-grid. C1b.3 owns retirement of Grid layout writes. Each child receives independent full binding review before implementation.
