# Architecture: Layout (elidex-layout)

- **Block layout**: `layout_block_inner()` handles block formatting context — width resolution, margin collapse (adjacent siblings, positive/negative), padding/border, vertical stacking. `is_block_level()` classifies display types.
- **Inline layout**: `layout_inline()` handles inline formatting context — text shaping, line breaking, line box construction.
- **Flexbox layout**: `flex.rs` implements CSS Flexbox Level 1 (simplified). `layout_flex()` entry point: box model resolution → item collection (`display:none` skipped) → `order` stable sort → line splitting (nowrap/wrap/wrap-reverse) → flexible length resolution (grow/shrink with frozen/unfrozen loop) → cross size resolution → main axis positioning (justify-content: 6 values) → cross axis alignment (align-items/align-self: stretch/flex-start/flex-end/center) → multi-line align-content distribution.
- **Phase 2 simplifications**: `baseline` alignment → `flex-start`, `flex-basis: content` → `auto`, `InlineFlex` treated as block-level.
- **Routing**: `block.rs` and `layout.rs` route `Display::Flex`/`InlineFlex` children to `flex::layout_flex()`.
