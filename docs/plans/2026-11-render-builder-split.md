# Extract paged display-list construction

PM 1001 puts this standalone change after G and before G′a-geo. The baseline render builder has 1,002 lines. Its public paged entry points and existing page output form the acceptance baseline. Full independent plan review precedes implementation.

## §1. Module boundary

Move build_paged_display_lists and build_paged_display_lists_interleaved and their page-only continuation, margin-box and generated-content helpers from elidex-render/src/builder/mod.rs to builder/paged.rs. Retain the original public paths through re-exports. Share root discovery and common drawing helpers through their existing owner. Audit callers with `git grep -n -E 'build_paged_display_lists(_interleaved)?' -- crates`. Preserve argument and return types, draw order, current sources of dimensions and margins, and fragment continuation behavior. G′a-geo introduces the selected geometry record after this extraction.

Move the tests that call `evaluate_content_value`, `margin_box_area` and `resolve_margin_box_style` into a test module under the paged implementation. Include the assertions on private style fields. Audit their references in `builder/tests/paged.rs` as well as the public callers; keep helper and field visibility private.

Correct the moved comments in the extraction commit. Webref confirms css-page-3 §5 “Page-Margin Boxes”, Table 1, defines the 16 boxes and their placement. Cite it at the three enumeration and geometry comments (baseline builder/mod.rs lines 495, 515 and 909). Section 4.2 “Page selectors” applies to selector matching. Describe the 12px constant at line 488 as the local renderer default and the struct at line 569 as resolved paint inputs; remove the selector citation from both.

## §2. Verification

Before moving functions, add a permanent characterization test calling the public `build_paged_display_lists_interleaved`. Give one fragmentable block two differently colored fixed-height children that fit individually in the page area but together force a break inside the block. Set `counter-increment: page 10` on that block alone and paint `counter(page)` in a margin box. The driver seeds this counter at outer scope before walking each page, so its increment survives scope exit. An element-local custom counter would be removed by `pop_scope` before margin painting.

Assert exactly two pages, the first child's painted rectangle on page one and the second child's on page two. Read the margin text glyphs using a deterministic test font with decimal digits: expect “11” on page one and “2” on page two (dropping the continuation set would paint “12”). Require the text commands on both pages. Pin the ordered body and margin commands and their page-local rectangle and glyph coordinates against explicit expected values on the unsplit implementation. Keep the fixture and assertions unchanged through extraction.

Run `cargo test --all-features -p elidex-render -p elidex-layout` with the new test before and after the move. Independently exercise ordinary and blank fragments through the precomputed driver; check body then margin painting on the ordinary page and margin painting alone on the blank page. Retain the screen builder tests. Record module sizes before and after; split page-only helpers into a further sibling in the same extraction if needed to keep the modules cohesive and below the threshold.

## §3. Spec coverage map

| Spec section | Step | Branch | Touch | Full enum? | User-input flow |
|---|---|---|---|---|---|
| css-page-3 §3 The Page Model | preserve page area and margins | ordinary walk, continued walk with prior token, blank walk bypass | both paged entry points | unchanged branching on layout flags and token presence, no enum migration | author content and page rules enter layout, which produces fragment flags and break tokens |
| css-page-3 §5 Page-Margin Boxes | paint margins after the body or blank bypass | matching rules with generated content on ordinary, continued and blank pages | page-only render helpers | preserve all 16 margin-box positions, selector checks and absent or empty content skips | author page rules supply position, style and content; render tests can construct the typed PageRule directly, and layout supplies the page number and blank flag |

Parser M17 and advisory MIN-2 assign the touch-time extraction here. Record test parity under M18; follow the umbrella §3 pre-push and PM landing gates under M19. Geometry selection and invalidation are verified in G′a-geo.
