# Extract paged display-list construction

PM 1001 puts this standalone change after G and before G′a-geo. The baseline render builder has 1,002 lines. Its public paged entry points and existing page output form the acceptance baseline. Full independent plan review precedes implementation.

## §1. Module boundary

Move build_paged_display_lists and build_paged_display_lists_interleaved and their page-only continuation, margin-box and generated-content helpers from elidex-render/src/builder/mod.rs to builder/paged.rs. Retain the original public paths through re-exports. Share root discovery and common drawing helpers through their existing owner. Audit callers with `git grep -n -E 'build_paged_display_lists(_interleaved)?' -- crates`. Preserve argument and return types, draw order, current sources of dimensions and margins, and fragment continuation behavior. G′a-geo introduces the selected geometry record after this extraction.

## §2. Verification

Run `cargo test --all-features -p elidex-render -p elidex-layout` before and after extraction. Compare paged display-list commands, offsets, page counts and margin boxes on the same ordinary, continued and blank-page fixtures. Verify the public interleaved and precomputed-page paths and the existing non-paged builder tests. Record module sizes before and after; split page-only helpers into a further sibling in the same extraction if needed to keep the modules cohesive and below the threshold.

## §3. Spec coverage map

| Spec section | Step | Branch | Touch | Full enum? | User-input flow |
|---|---|---|---|---|---|
| css-page-3 §3 The Page Model | preserve page box output during module extraction | ordinary and continued pages | render builder paged entry points | existing paths | existing paged fixtures |
| css-page-3 §5 Page-Margin Boxes | preserve existing margin-box painting | generated margin content | page-only render helpers | existing paths | existing stylesheet fixtures |

Parser M17 and advisory MIN-2 assign the touch-time extraction here. Record test parity under M18; follow the umbrella §3 pre-push and PM landing gates under M19. Geometry selection and invalidation are verified in G′a-geo.
