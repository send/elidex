# Publish selected geometry with each printed page

G′a-geo 3b.4 owns F5 and closes F6. The 3b.3 plan landed at 73363beb. Its ordered content and blank entries supply the identity and geometry for both builders.

## §1. Publish each physical page

At 73363beb both public builders return PagedDisplayList with a vector of display lists and a single physical size taken from the default page context. The fragment builder separately resolves margins and dimensions from the context. A caller cannot retrieve the sheet used for each page of a mixed-size result.

Give the result an immutable geometry table built from the selected entries for this run. Each ordered output record pairs its display list with a reference into that table. Keep the index private to the result and expose a page accessor that resolves the reference against its owner. Publish the selected physical size, margins, content origin and area in CSS pixels. Carry the printed ordinal, selected side and blank/content kind from the same 3b.3 entry that supplied the geometry. PM 1016 F5 gives selector matching and first-area preparation one run identity seeded by progression and the propagated first-side request. Both builders use the resulting entry for sheet selection, margin markers and printed counters. Printed page 1 can be left. Publish records and their table together, including for empty input. Keep page_count and is_empty based on ordered output records. Migrate users of the global page_size and direct list indexing, including test fixtures and the shell print pipeline.

Use one result construction path for build_paged_display_lists and build_paged_display_lists_interleaved. The fragment path needs the corresponding prepared geometry with each fragment, including blank fragments. During interleaved replay, pair the list with the geometry from that same planned entry. Feed its physical dimensions and margins to margin-box emission. Body coordinates already include the layout origin. Preserve draw order and avoid a second translation. A blank emits its selected margin content. Only the content-layout progress calculation may use a minimum extent. A zero-height sheet remains zero in the public result.

Under PM 1016 F2, Block, Flex and Grid feed the shared chooser ordered boundary values. Definitive layout supplies the winning combined requirements and candidate coverage together with actual item outcomes to 3b.3. Only committed plan entries produce public records. Render consumes the resulting sequence directly. Rendering a blank or replaying a content entry never re-applies the break event. Probes and rejected candidates contribute no record or margin marker.

Own the geometry for the lifetime of each returned result. A subsequent invocation with a changed stylesheet and a shorter page sequence may reuse local indices. Both the retained first result and the new result must continue to resolve every page to its own geometry after the context and plan have been dropped.

## §2. Crossing checks

| Crossing | Fixture through public entry points | Observation |
|---|---|---|
| Chosen boundary × physical output | At an internal Grid row boundary after A, combine break-after:right with break-before:page on B. In default LTR, use a 200×100 sheet with 10px margins, a 140×100 first sheet with 20px left and 10px other margins, and a 120×60 blank sheet with 5px margins. | Exactly three records: first-right content with 110×80 at (20,10), left blank with 110×50 at (5,5), then right content with 180×80 at (10,10). Inspect the blank marker bounds and absence of body on page 2. Change B’s request to left: the later competing side wins, yielding two content records and no blank marker. |
| Initial side × public geometry | In LTR start with a propagated verso request, then an ordinary page break. Use distinct :first:left and :first:right sheet sizes and margin markers. Repeat with recto and the ordinary default. | Verso starts with printed page 1 first-left, followed by page 2 right. Recto and default start first-right, followed by left. Assert both builders publish the selected sheet and matching margin marker for each page, with counters 1/2 and 2/2 and no leading blank. |
| Layout origin × drawing | Set body { margin: 0; padding: 0 } and use one painted block with zero margin, border and padding. Give the page asymmetric margins and a first-page size override. Exercise the ordinary fragment builder with fresh layout and the interleaved builder separately. | Compare the first painted box bounds with the expected content origin in each result. The margin marker uses the physical page coordinate system. A double translation displaces the box from its expected position. |
| Repagination × retained result | Change B's request to left and change the content-page width. Keep both public results while dropping the input contexts. | The new result has two content pages: the first retains its explicit first-page size and the second uses the new default width. The first still resolves three pages including the original blank. Check the selected geometry of every individual record. |
| Degenerate geometry × identity | Reuse the 3b.3 zero-height content fixture and separately an empty document. | The recorded content sheet has physical height zero, the selected origin and its content selector. Empty output reports zero pages and exposes no geometry references. |

Preserve the interleaved path's existing per-generation body and continuation-counter checks. The ordinary builder reads already-laid-out ECS boxes, so drive its multi-page geometry and blank-margin checks from the supplied fragment metadata and use a fresh single-page layout for its painted-origin assertion. Verify the shell caller returns the interleaved result intact.

## §3. Spec coverage map

| Spec section | Step | Branch | Touch (compile/dispatch site) | Full enum? | User-input flow |
|---|---|---|---|---|---|
| css-page-3 §7.1 Page size | publish the selected sheet | default, first, blank and zero height | page plan to public record | selected sizes | yes |
| css-page-3 §4.2 Page selectors | carry first and spread identity | first-left, first-right and following side | planned entry to both public builders | first and side selectors | yes |
| css-page-3 §4.2.3 Blank-page pseudo-class | publish accepted blank identity | forced blank and zero-area content | planned entry to page record | content and blank | yes |
| css-page-3 §5 Page-Margin Boxes | emit selected margin geometry | ordinary and blank-only marker | public builders to margin-box emission | selected page margins | yes |
