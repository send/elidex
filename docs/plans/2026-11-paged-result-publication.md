# P-C: Publish complete or explicitly partial paged results

## §1. Real consumers

This plan absorbs the proposed #11-paged-render-run-completion. The precomputed and interleaved public builders in elidex-render/src/builder/paged.rs and callers supplying fragments or consuming page_count need P-B's accepted run identity, terminal status, saved inputs and placement policies. A supplied vector length cannot certify a final total. Drain a progressing tail before publishing a final document. On Incomplete return the accepted prefix as explicitly partial, without a resumable tail or final-total-dependent margin boxes. Both paths must distinguish this from a genuinely complete empty or exactly-at-limit document. Precomputed geometry must be paired with the matching selected source/generation; reject incompatible inputs. Interleaved count and paint must agree on owner, geometry, policy, page identity and accepted end; disagreement invalidates publication.

3b.3 supplies the sole ordered page plan after the Flex/Grid prerequisites. Verify that the initial request chooses the first printed side without a leading blank, conflicting side requests respect flow order, and a compound page/column transition discharges both at column one. Replay uses the same recorded transition and geometry through both builders.

## §2. Review inputs and fixtures

- R1 F4 (retry fixture from P-B’s R2-4): normal success, rejected normal then accepted fresh attempt, and repeated stall with earlier red prefix. A rejected blue atom never paints. Assert accepted generation and selected terminal set on replay, no extra zero-room page, no fabricated final counter. Run equivalent precomputed input as well.
- R1 F6 / R2-6: 999/1000/1001 page-filling children; blue sentinel only on its intended page, final counters agree. Include partial with the same length, a blank side page with only its number, a visible line and a retreated suffix. Test each builder explicitly: capture a supplied fragment/selection sequence for precomputed; generate it through actual dispatch for interleaved. Archive base, provider and adopted output separately. The umbrella gives the measured base controls and the known missing blue on page 1001 in interleaved mode.
- Consumer references from R1 F2/F3/F5, PM 1180 H1/H2, R2-1/2/3/5: carry selected membership through paint. Repeat compound-side parity, independent float text, completed span with outstanding track geometry, overflow versus a real orthogonal gap, excluded/depth-limited subtrees, and deferred atom. Assert page-local command IDs/coordinates and empty complement for final success. No new terminal writer or break chooser belongs in Render.

## §3. Spec coverage map

| Spec section | Contract |
|---|---|
| css-page-3 §3.3, 6.1 | Side selection and page counters use the accepted sequence. |
| css-break-3 §2.1, 4.3 | Paint the selected fragments of each flow and the discharged boundary once. |

Switch P-C and its production layout readers together after 3b.3 and the nested Flex/Table receivers are available. The partial-result API and retry-generation assertions are engine contracts.
