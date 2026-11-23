# Flex omit-initial program

Flex and flex-flow declared getters will serialize the winning typed longhands while preserving their meaning under reparse. This is the scope record for the Flex increment of #11-style-shorthand-expand. PM 959–977 establish the cuts below. R3 is the final umbrella review: cuts, landing order, crossing invariants and finding ownership are its gate. Each implementation PR still needs a separate full plan review. Acceptance examples and source inventories belong in those child plans.

## §1. Cuts and landing order

| Order | PR | Direct predecessors | Delivery boundary |
|---|---|---|---|
| 1 | T · substitution | — | One token-preserving receiving-property reparse and one winner-selection primitive using existing cascade order. Publish computed custom properties before inheritance. |
| 2 | G · Flex grammar | T | Share Flex factor and basis parsing; move the strict unit lookup, length, length-percentage and calc grammar to elidex-plugin. Accept the CSS Values §6 subset resolvable with the existing ResolveContext (table below). Reject a dimension requiring any deferred context during parsing, including inside calc(). |
| 3a | Render builder split | G | Extract the paged display-list builder into a sibling module with identical public behavior. Standalone touch-time split for the oversized builder. |
| 3b.1 | G′a-geo · print style preparation | T, G, render builder split | Style prepares candidates from one sheet snapshot with grammar context and cascade metadata, ranks all geometry inputs by the same page specificity and cascade order, and resolves page-local winners against the immutable root map. Supply resolved inputs to a pure geometry evaluator. Use UA geometry for cyclic inputs, select the first page area, then compute document style with that area and a separate media-query reference. |
| 3b.2 | G′a-geo · source cursor | 3b.1 | Resume inline content by stable source position in block and mixed-child layout. Define the shared cursor type and advancement rules for Flex and Grid. Cover Multicol’s nested-token forwarding with a regression test. |
| 3b.2f | G′a-geo · Flex item continuation | 3b.2 | Use 3b.2’s source cursor type and advancement rules when wrapped Flex lines regroup. Run the four 40px items at 80→40px and 40→80px and verify that each item is emitted once in order. |
| 3b.2g | G′a-geo · Grid item continuation | 3b.2, 3b.2f | Use the shared cursor when auto-repeat changes row membership. Run four auto-placed items in repeat(auto-fill, 40px) at 80→40px and 40→80px, compare explicit placement, and verify the remaining state of a spanning child. |
| 3b.3 | G′a-geo · pagination plan | 3b.1, 3b.2, 3b.2f, 3b.2g | Pagination consumes typed break-boundary events once, determines blank/content entries from parity, then selects geometry for each entry. Replay the same entry order, source positions and geometry for counting and layout for paint. Preserve actual dimensions separately from the minimum 1px fragmentation progress extent. |
| 3b.4 | G′a-geo · public page geometry | 3b.3, render builder split | Both public builders return per-page output records referencing the immutable geometry table owned by their result. Verify sheet dimensions, content origins and margin boxes on blank and content pages across repagination. |
| 3c | G′a · lengths with a definite reference | 3b.4 | Migrate Text, table border spacing and Multicol length properties with their computed and used readers. Each accepted calculation ships with its evaluator. |
| 4 | N · numeric semantics | G′a | One lexeme conversion, arithmetic and simplification implementation in elidex-plugin for all consumers, including style and media. |
| 5 | C1 · computed representation | N | One computed length-percentage payload for Dimension and FlexBasis, with ownership, equality and all mandatory reader changes including transform-origin. |
| 6 | G′b · reference-dependent consumers | C1 | Migrate grid, float, transform, box and background with their reference-size readers. Retire remaining duplicate decoders and legacy plugin wrappers here. |
| 7 | C2 · Flex used basis and generic animation | C1, G′b | Resolve the Flex used basis and content probe consistently. One interpolation function for all properties combines through N and publishes the simplified sampled value. |
| 8 | D · declaration block activation | T | Activate rule/inline read, mutate, reflect and cascade together. Shared Declaration, winner, pending and provenance payloads live in elidex-plugin. Keep a single conversion for existing string handlers. |
| 9 | H · handler signature | D, N | Move all handlers/helpers to typed pairs together, compare typed initials and remove D’s string conversion. |
| 10 | S · numeric serialization | H, C2, G′b | Format declared and resolved values from typed inputs, including sampled values; preserve media breakpoint semantics. |
| 11 | F · Flex omission | G, S | Implement the two family serializers using the common coordinator, typed comparison and public formatter. |

G′ is split by the reader’s dependency, following its breadth warning and PM 965/971. G′a-geo and G′a compute with available context; G′b waits for C1 where percentages must survive to layout or paint. The page and property readers call G’s grammar, with the category required by each property. The acceptance condition is **one implementation of CSS length and length-percentage grammar**, including @page. Moving a consumer includes its direct, substituted (where applicable) and used-value paths. C1 precedes newly admitted inputs requiring its payload. N’s common semantics also apply to consumers switched later in G′b.

PM 975, as corrected by PM 977, selects the following unit boundary at G. The unit families follow [CSS Values §6.1](https://drafts.csswg.org/css-values-4/#relative-lengths) and [§6.2](https://drafts.csswg.org/css-values-4/#absolute-lengths).

| CSS length units | Context or conversion | Decision at G |
|---|---|---|
| px, cm, mm, Q, in, pt, pc | Fixed ratios to px | Accept |
| em, rem | Element or root font size | Accept |
| vw, vh, vmin, vmax and lvw, lvh, lvmin, lvmax | The same large viewport width and height | Accept |
| ex, rex, cap, rcap, ch, rch, ic, ric | Element or root font metrics | Reject at parse |
| lh, rlh | Element or root line height | Reject at parse |
| vi, vb, lvi, lvb | Large viewport along the box’s own inline or block axis (initial writing-mode when there is no element) | Reject at parse |
| svw, svh, svi, svb, svmin, svmax; dvw, dvh, dvi, dvb, dvmin, dvmax | Small or dynamic viewport, plus box writing mode for logical axes | Reject at parse |

If the G′b child review calls for another split, PM approves the revised cuts, prerequisites and reassignment of every affected §4 ID before implementation. Each part lands its grammar and readers together. The last part removes the remaining wrappers. C2 and S follow **all** G′b parts.

The complement audit covers the entire code base at c3c78070. Inspect token readers, unit-string tables and text-based parsing, then trace callers. Length properties and descriptors are in scope; color channels, times, angles, grid flex units and non-CSS attribute grammars have different domains. Per-group dispositions, unit coverage and calc acceptance are in G, G′a-geo, G′a and G′b.

```sh
git grep -n -E 'Token::(Dimension|Percentage)|expect_dimension|expect_percentage|length_to_px|parse_length_unit' c3c78070 -- crates
git grep -n -Ei 'parse_[[:alnum:]_]*length|length[[:alnum:]_]*_to_px|"(px|em|rem|pt|pc|cm|mm|in|q)"' c3c78070 -- crates
```

Repeat both commands with HEAD at the final G′b landing head. Classify remaining hits as the common grammar, its callers/tests, or a named different grammar or non-CSS use (including unrelated string literals). Require zero independent in-scope decoders and zero lenient unknown-unit fallbacks. Run G’s direct, substituted and @supports controls for every deferred unit family at this head as well: the parser must reject those dimensions even where a fallback arm contains no unit literal. Separately, C1 measures the computed-type population with `git grep -l -w Dimension c3c78070 -- crates`.

## §2. Crossing invariants

| Edge | Contract at the consuming head |
|---|---|
| T → G, G′a-geo, G′a, G′b | Reparse the preserved sequence against the same grammar as a direct declaration. A direct parse failure and invalidation after successful cascade have distinct fallback behavior. |
| G → G′a-geo → G′a → N → C1 → G′b | The unit type and calculation tree retain lexical precision, numeric category and percentage dependence. Each group admits the §1 supported units with a working reader and rejects deferred units at parse. |
| N → C1 → C2 → S | N owns simplification. C1 retains unresolved references, C2 passes interpolation results through N before storage, and S renders that same computed or sampled payload. |
| G′b → C2 | Before generic interpolation can produce mixed length-percentage samples, transform translation and background-position must both resolve them against their actual reference sizes. |
| G′b → S | S serializes the typed values supplied by the migrated readers, preserving unresolved percentages where their computed type requires it. |
| T → D | T supplies the same-block winner primitive using importance and stable declaration position. D calls it on block construction or mutation, including standalone rules before any element cascade. CSSOM reads that result. Cascade consumes the same block winners with their source metadata and ranks across blocks; substitution uses the winning member’s position. |
| T → D → S | Custom-property and pending token identity survives mutation, reflection, rehydration and clone. Public rounding never changes the authoritative value on the original entity. |
| D → H; N → H → S | D owns the typed block. H removes its temporary string conversion and compares unrounded typed values. S owns display precision and preserves each caller’s serialization context. |
| G, C2, D, H, S → F | Omission preserves grammar, typed meaning, public precedence and used basis. PM 928’s zero-basis, tie-break, WPT cross-check and no-regression decisions apply throughout. |

D’s single materializer/mutator lives in elidex-style. InlineStyle on the entity and the parsed rule block share plugin-tier payloads. CSSOM and the style walk call that operation; external attributes invalidate it and clone starts from copied attributes. Future rule mutation under #11-css-rule-mutation is downstream of this contract and outside D.

## §3. Spec coverage map

| Spec section | Step | Branch | Touch | Full enum? | User-input flow |
|---|---|---|---|---|---|
| css-values-5 §A Arbitrary Substitution Functions | substitute and reparse | token boundaries, pending members | T → D | supported substitutions | yes |
| css-cascade-4 §6.1 Cascade Sorting Order | rank declarations | block order, importance, cross-block rank | T → D | supported cascade | yes |
| CSSOM §6.6 CSS Declaration Blocks | create from attribute / serialize block | inline and rule | D / regrouping owner | relevant branches | yes |
| WHATWG DOM §4.4 Interface Node | clone a node via cloning a single node | copied attributes feed CSSOM block creation | D | elements | yes |
| css-flexbox-1 §7.1 The flex Shorthand | expand or omit | typed factors and basis | G → F | supported grammar | yes |
| css-values-4 §5.4 Numbers with Units | dimension syntax | numeric token plus identifier | G | syntax | yes |
| css-values-4 §6 Distance Units | length category and units | absolute and relative lengths, bare zero | G → G′ | length grammar | yes |
| css-page-3 §7.1 Page size | resolve size descriptor | one/two lengths, page context | G′a-geo | length branch | yes |
| css-values-4 §10.10.1 Simplification | simplify typed tree | computed, sampled or declared | N → C1/C2/S | supported math | yes |
| css-values-4 §10.14 Combination of Math Functions | interpolate | sampled type and reference base | C2 → S | length-percentage | yes |
| CSSOM §6.7.2 Serializing CSS Values | serialize a CSS value | representability and omission | H/S/F | supported families | yes |

Child plans own their complete algorithm/branch maps, including mediaqueries-4 §§1.2–1.3 in N/S. Every child gets its own full review before implementation, scoped tests, pre-push CI, PM push clearance, single-pass post-push review and explicit merge-at-head clearance. PM decides additional scope or standalone touch-time splits.

## §4. Finding homes

Finding text is indexed in the lane handoff directory `/Users/kazuaki/repos/send.sh/elidex-pm-trial/handoff/codex-lanes/l1/` under `gate-r2-9853348d.md`, `gate-r2-c2f27760.md`, `gate-r2-ab539f10.md`, `gate-umb-df8f8336.md`, `gate-umb-1696e3b8.md` and `gate-umb-f73c77eb.md`. IDs are local to the indicated review head. An arrow assigns the producing contract and its consumer check.

| Flex @ 9853348d | Responsible slice |
|---|---|
| F1, F7 | Umbrella |
| F2, M14 | S |
| F3 | D → S |
| F4, F5 | H → F |
| F6 | N → S |
| M8, M11 | S |
| M9, M10 | F |
| M12 | Umbrella |
| M13 | F |

| Parser @ c2f27760 | Responsible slice |
|---|---|
| F1, F2 | Umbrella → G/G′a-geo/G′a/G′b/N/C1/C2 |
| F3, F10, M20 | N |
| F4, F6, F7 | C1 → C2 |
| F5, F8 | G |
| F9, M13, M14 | G |
| F11, M12, M15 | G → G′a-geo/G′a/G′b |
| M16, M17, M18 | G/G′a-geo/G′a/G′b/N/C1/C2 |
| M19 | Umbrella → G/G′a-geo/G′a/G′b/N/C1/C2 |

| Declaration order and token @ ab539f10 | Responsible slice |
|---|---|
| F1 | T → D |
| F10 | D |
| F2, F4, M13, M16 | T |
| F3, M9, M19, M24 | T |
| F5, M12 | D |
| F6 | Umbrella → H |
| F7, F11 | H |
| F8, M22 | D |
| M14, M15, M17, M18, M20 | D |
| M21 | T |
| M23 | H → F |
| M25 | T, D |

| Umbrella R2 at df8f8336 | Responsible slice or record |
|---|---|
| F1, M9, M15 | G vocabulary → G′a-geo (page), G′a (property readers) and G′b migration |
| F2, M21 | Umbrella split/order → G′a-geo/G′a/G′b reader acceptance |
| F3 | T → G′a-geo/G′a/G′b |
| F4 | C2 → S |
| F5, M11 | T → D |
| F6, M10, M12, M13, M16, M22 | D |
| F7, F8, M18, M20 | Ledger decisions below; D owns the inline reference update; G′b owns translation |
| M14 | C1 |
| M17 | N → S |
| M19 | G / coverage map |
| M23 | Corrected original finding homes above |

R3 child homes at 1696e3b8: D (C-1, C-8, C-10–C-13), N (C-2, C-15), G′a-geo (C-3), G′a (C-5, C-16), G (C-4), S (C-6, C-14), F (C-7), H (C-9), T (C-13), G′b (C-16).

PM 1008 splits G′a-geo into §1 rows 3b.1–3b.4. Each child needs an independent full plan review before implementation and its predecessor implementations before landing code. For the G′a-geo entries above, the page portions of §4 have these homes.

| Review input | Page findings and owners |
|---|---|
| Parser @ c2f27760 | F1/F2 and F11/M12/M15 go to 3b.1 for grammar and readers, with actual consumption checked in 3b.3/4. Each child owns its applicable M16–M19 coverage, structure and gates. The standalone 3a extraction retains M17 for the builder split and M18/M19 for parity and landing checks. |
| Umbrella R2 @ df8f8336 | F1/M9/M15 and F3 go from G/T to 3b.1. The umbrella owns the F2/M21 cut and order, with page-reader acceptance through 3b.1 → 3b.3 → 3b.4. |
| Umbrella R3 @ 1696e3b8 | C-3 belongs to 3b.1 for each margin side’s grammar, cascade and evaluation, 3b.3 for page axes and breaks, and 3b.4 for public output. N retains C-2 for page and property readers. |
| G′a advisory @ 17e44ba3 | IMP-1 goes to 3b.3 → 3b.4, IMP-2/6 to 3b.1, and IMP-7 to 3b.1 → 3b.3/4. G′a retains IMP-3–5 and MIN-1. Each reader owns its IMP-8 spec rows and MIN-2 tests after the 3a extraction. Each PR owns MIN-3. The revised order settles MIN-4. |
| G′a-geo binding @ 6ffdfec2 | 3b.1 owns F1/F2/F7 and the bare-orientation control F9. 3b.2 owns F3’s cursor contract, with Flex and Grid consumers in 3b.2f and 3b.2g. 3b.3 owns F4/F8. 3b.4 owns F5 and closes F6 across the selected-geometry children. |

| 組 | producer → consumer と判別観測 |
|---|---|
| 置換 × pagination | 選択ページの margin 勝者 → fragmentainer。first だけの変数を変えると first の break が移り、通常ページの margin は一定。 |
| 置換 × paint | page-local 勝者 → geometry table → 両 builder。first でだけ勝者を無効にしたとき unset の原点になり、以前の負けた宣言を描画しない。 |
| 置換 × invalidation | 更新 sheet と不変 root map → style の新 snapshot。同じ変数名の first/通常ページで valid → invalid にした側だけが unset になる。 |
| pagination × paint | blank/content entry → 両出力。ページ数・順序・物理寸法・原点が一致し、blank の margin box と続きの先頭語を確認。 |
| pagination × invalidation | 更新した first 入力 → 新しい page plan。境界を跨ぐ箱の page 番号と後続の開始 cursor が変わり、旧 plan を再利用すると失敗する。 |
| paint × invalidation | 新旧 layout 結果 → 描画 record。新しい margin box と原点を得た後も、保持した旧結果の table と描画 record が旧寸法を返す。 |

L2 owns line existence and decoration, character transformation, list UA rules and counter order. The shared paths are tracked under #11-line-box-decorated-inline-content, #11-empty-line-box-inline-mbp, #11-dom-cr-is-not-segment-break and #11-list-counter-baking-ua-rule-and-hints. L1 owns preservation of source position, page events and geometry replay.

Focused re-check at f73c77eb: C2 owns V-C1, T and G own V-C2, and G owns V-C3. PM 977 accepted the umbrella at this head.

**Proposed slot `#11-css-length-resolution-contexts`.** **Owner**: L1. **Re-eval-date**: 2026-12-15. `crates/core/elidex-plugin/src/traits.rs` (`ResolveContext`) lacks the font metrics, line heights, writing mode and differentiated viewport sizes required by CSS Values §6.1. Today the lenient css_resolve::parse_length_unit path maps unknown units to px, so a dimension requiring a missing context can silently resolve as pixels. G will make such direct declarations invalid until the corresponding context exists. Each missing context introduces a separate invariant governing which value is available and when it updates. PM 975 defers those axes to this slot for PM registration when the umbrella lands.

**Ledger.** #11-css-declared-value-serialization-fidelity is out of program scope. S tracks #11-cssvalue-list-separator-fidelity and checks preservation of current separators when formatting numeric leaves. H reports group (k) of #11-core-browser-parity-deviations via PM. Existing re-evaluation dates stand. On December 15 PM chooses extend/start/close per blocker for #11-shorthand-direct-decl-cascade-order (D), #11-shorthand-omit-initial-csswide-omission (F) and #11-css-wide-revert-keyword (H).

**Proposed parent-slot wording for PM registration.** **Owner**: L1. **Re-eval-date**: 2026-12-15. Also revisit when another shorthand family lands. Under #11-style-shorthand-expand track one declaration-block regrouping algorithm for both rule and inline cssText. RuleStyleCssText/push_declaration and InlineStyle::css_text emit individual longhands. CSSOM §6.6 “serialize a CSS declaration block” step 4.3 invokes “serialize into a shorthand form”; its steps 2.5 and 2.7–2.8 check priority and representability. The parent remains open for this shared block operation after individual family getters land. D updates the inline source citation to the PM-registered home. Block-wide candidate selection spans families and governs cssText enumeration; F implements the individual Flex family getter.

**Translation handoff.** **Owner**: L1. **Re-eval-date**: 2026-12-15. On that date PM checks G′b landing evidence and decides whether the former deferred draft can close. PM 975 records the separate approval required by PM 961 (2), derived from PM 965 F2 and PM 971 F2, for X/Y and Z translation calc and perspective length calc in G′b. The PM 965 F4 list of deferred transform features is empty. Direct transform parsing currently rejects mixed calc offsets and resolve_translate_value falls through to zero for a calc value. G′b owns admission plus reference-box evaluation after C1. This supersedes the proposed deferred transform slot; PM may close that draft against G′b’s landing evidence.

## §5. Review boundary

The original local review inputs remain available at 9853348d (Flex), c2f27760 (parser) and ab539f10 (token/declaration order). The three November gates retain 14, 20 and 25 homes above. The July shorthand program remains the parent for other families. After R3, submit children individually against the approved program. Keep 9853348d and dca05c9e (G′a draft) pinned as immutable inputs. Before committing the F and G′a drafts, branch their worktrees at the pinned heads under new child branch names. The following draft locations use the L1 workspace as their root.

| Slice | Worktree / plan |
|---|---|
| T | `flex-token/docs/plans/2026-11-shorthand-token-substitution.md` |
| G | `flex-grammar/docs/plans/2026-11-flex-shorthand-parser-prerequisite.md` |
| Render builder split | `render-builder-split/docs/plans/2026-11-render-builder-split.md` |
| G′a-geo binding input @ 6ffdfec2 (before PM 1008) | `page-geometry/docs/plans/2026-11-selected-page-geometry.md` |
| G′a | `length-consumers/docs/plans/2026-11-length-percentage-consumer-migration.md` |
| G′b | `flex-length-used/docs/plans/2026-11-length-percentage-used-readers.md` |
| N | `flex-numeric/docs/plans/2026-11-css-numeric-evaluation.md` |
| C1 | `flex-computed/docs/plans/2026-11-computed-length-percentage.md` |
| C2 | `flex-used/docs/plans/2026-11-flex-used-basis.md` |
| D | `flex-declarations/docs/plans/2026-11-typed-declaration-block.md` |
| H | `flex-typed-handler/docs/plans/2026-11-typed-shorthand-handlers.md` |
| S | `flex-formatting/docs/plans/2026-11-css-numeric-formatting.md` |
| F | `flex-omit-initial/docs/plans/2026-11-shorthand-flex-omit-initial.md` |
