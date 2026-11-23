# Flex omit-initial program

**Decision requested.** Approve the following ten implementation PRs under PM 959, 961, 962, 965 and 966. The end product is a declared-value getter for `flex` and `flex-flow` that selects a meaning-preserving shorthand from the actual winning typed declarations. An unrelated inline mutation must preserve those declarations' computed effect. This umbrella is the scope record for Flex under the July shorthand handler-dispatch program. The parent ledger item is #11-style-shorthand-expand. PM 928/929 established the original prerequisites. The three November 23 binding reviews at 9853348d, c2f27760 and ab539f10 reopened the plans before implementation.

## §1. Landing order and ownership

The table gives the proposed landing order. The predecessor column lists direct technical dependencies. Slices with no path between them can be designed independently. Land this documentation and its July staging pointer before T. Approve this umbrella first, then require a separate full five-axis /elidex-plan-review on each slice's own memo and head before implementation. The three reviewed heads are source material for those rewrites.

| Order / PR | Requires | Deliverable and boundary |
|---|---|---|
| 1 T — token substitution | — | One component-value substitution and receiving-property reparse path on existing stores. Preserve token boundaries and lexemes, distinguish absent/empty/nonempty fallbacks, handle cycles and limits, compute before inheritance. Own detection, computed custom-property readers and removal of guessed shorthand expansion. Reuse the cascade’s stable order when reparsed members collide with independent longhands (PM 966). |
| 2 G — Flex grammar | T | Share factor and basis parsing across shorthand, built-in longhand and registered handler. Move the existing calc grammar from elidex-css into elidex-plugin, migrating current strict imports and Flex entry points. Resolve bare zero, group order, defaults, number-typed calc factors, length/percentage basis and rejection of negative literals and unknown units. Substituted declarations pass the same grammar and three-longhand acceptance as direct input. |
| 3 G′ — strict length consumers | G | Remove the plugin’s lenient unit lookup and token grammar; migrate every caller and equivalent local token match in the inventory below. Each changed property path has a direct acceptance test at this head. |
| 4 N — numeric conversion and arithmetic | G′ | One original-lexeme conversion and typed calc evaluator for every consumer, including media. Specify storage bounds, signed zero, all five calc keywords, IEEE intermediate results and each caller's top-level range policy. Own the single CSS Values simplification algorithm in elidex-plugin. Own changes in accepted numeric inputs and arithmetic for existing non-Flex callers. |
| 5 C1 — computed length-percentage representation | N | Share one computed payload across Dimension and FlexBasis. Define cloning/borrowing after loss of Copy and equality on canonical computed values, including stable equality for repeated exceptional inputs after range conversion. Migrate all readers and writers to compile and preserve semantics, including both origin axes and the animation apply path. |
| 6 C2 — Flex used basis and interpolation | C1 | Resolve explicit basis and auto/main-size through the C1 payload. Share the definite/indefinite content decision between base sizing and the content probe. Sample length/calc and calc/calc animation using that representation. |
| 7 D — declaration order | T | Activate the authoritative typed per-longhand winner and provenance together in rule and inline stores. Both getters/priority, enumeration, inline set/remove/cssText, reflection, hydration, clone, attribute invalidation and computed cascade switch together. D consumes T’s cascade winners and owns public block order and provenance. Existing family handlers receive strings through one coordinator conversion. |
| 8 H — typed shorthand handler contract | D, N | Replace the string-pair CssPropertyHandler serialization signature with ordered typed pairs for every handler and coordinator entry point at once. Delete D’s coordinator conversion to string pairs. Migrate Box, Table, Multicol, helpers and serialize_parsed. Compare owner-provided typed initials before formatting. Own the common survivor rule and its Multicol consequences. |
| 9 S — shared numeric and calculation serialization | H, C1 | Separate public decimal formatting from typed storage and media breakpoint identity. One arithmetic-tree serializer supplies canonical operator and child order with explicit leaf context. Cover every to_css_string caller, rendering N’s simplified trees, finite roundoff and non-finite spellings. H precedes public rounding so existing handlers already receive typed values. |
| 10 F — Flex omission | G, C2, S | Add FlexHandler selection for both shorthands on the existing coordinator path. Reuse H for typed dispatch and S only after selection. Preserve declaration readback, round-trip at exposed precision and the distinction between zero length and zero percentage. D and H reach F through S. |

The slice author attaches its local test matrix, source inventory and named spec steps to its own memo. The PM receives a scope question whenever a proposed finding adds a crate, consumer group or invariant axis. The author records the proposed owner and waits for a decision before adding that work. For each implementation head use the lane's single-pass post-push Codex review, PM verdict and explicit merge-at-head instruction.

### Parser ownership and base inventory

G keeps declaration production and shorthand group parsing in elidex-css declaration/flex.rs. The direct FlexHandler in elidex-css-flex and the built-in longhand parser call the shared factor/basis grammar in elidex-plugin. The strict clients in elidex-css (values, declaration, box_model, font, grid, misc and media) change imports at G and get direct regression controls there. N moves common arithmetic and simplification to elidex-plugin; elidex-css media/eval.rs::resolve_calc supplies its f64 media context, while style supplies its f32 context. Test the actual breakpoint on both sides after migration.

Reproduce the caller sweep against the common code base. The first search finds eight files. The default-arm search also finds the plugin fallback and five background fallbacks (gradient twice, position once, shorthand twice). Three of the eight named-unit files belong to the already strict elidex-css implementation (values, declaration/misc, media/parse); css_resolve defines the lenient function. Grid, Multicol, Text and Transform use it directly. The other searches measure wrappers and the complement of the name-based list, including locally decoded tokens.

```sh
git grep -l -w parse_length_unit c3c78070 -- crates
git grep -n -E 'parse_(non_negative_length|length_or_percentage|length_percentage_auto)' c3c78070 -- crates
git grep -n -E 'Token::(Dimension|Percentage)|expect_percentage|expect_dimension' c3c78070 -- 'crates/css/elidex-css-*/src/*.rs'
git grep -n -E '_ => (Ok\()?([[:alnum:]_:]*LengthUnit::Px|CssValue::Length)' c3c78070 -- crates
git grep -l -w Dimension::Percentage c3c78070 -- crates
```

| G′ owner or client | Numeric routes and direct tests at G′ |
|---|---|
| Plugin / Flex | Remove the unknown-unit-to-Px default and duplicate length/percentage decoder after redirecting wrappers. G already moved the Flex client. Confirm no surviving call into the old implementation. For each route below, pair an accepted supported unit with 10bogus (formerly decoded as px, now rejected); inspect typed output and full consumption. |
| Grid and Float | Grid plain tracks, minmax breadths and fit-content use the checked grammar; keep fr local to flexible tracks. Float vertical-align retains signed offsets and keywords. Exercise direct registry handlers as well as built-in declaration controls. |
| Transform and Table | Test translate X/Y, Z length-only, transform/perspective origins and perspective’s nonnegative length. Test both internal border-spacing axes via TableHandler and one/two lengths via the public grammar. Keep the sign and percentage restrictions specific to each property. |
| Text and Multicol | Text font-size and line-height duplicate dimension/percentage parsing, including the grid-like text helper. Keep unitless line-height numeric. Spacing is length-only but calls the replaced unit function. Multicol column-width and column-rule-width do likewise. Migrate and test all of them, including keyword controls and percentage rejection on length-only routes. |
| Box | The auto/none, auto and nonnegative wrappers change the numeric paths for dimensions, margins, padding, radii, gaps and border widths. Test each changed property route via BoxHandler, including signed and nonnegative controls. The opacity branch consumes a number/percentage and returns a scalar, outside this length grammar. |
| Background | Size longhands call the nonnegative wrapper. Position, shorthand position/size and gradient centers/stops also contain local unit switches with unknown-to-px defaults. Migrate the length alternatives and test longhand, shorthand and gradient input directly. Keep angle/percentage stops in their appropriate context. |
| Other token matches | Angle decoding in transforms/gradients and animation time dimensions use different unit sets. They do not consume the replaced length parser. Re-run the sweep after G and require a disposition for every new match before G′ review. |

G′ is one migration because every change removes an alternative decoder in favor of the same strict grammar. Existing property categories stay with their owners. Numerical evaluation, computed representation and new used-value features have separate PRs. G′ closes only when the sweep finds no lenient implementation and every changed route rejects unknown units.

### Component ownership at activation

| State | Single writer and invalidation |
|---|---|
| Computed custom properties (T) | The style resolver publishes owned immutable token sequences or guaranteed-invalid on each element’s ComputedStyle. The table layout writer inherit_for_anonymous copies the computed map to anonymous styles. Both computed getter paths serialize it. |
| Inline declarations (D) | Put the authoritative typed winners, pending token sequences and group provenance on the entity’s InlineStyle. A registry-aware elidex-style operation is the sole typed-block materializer/mutator, called before cascade collection with mutable ECS access and by dom-api CSSOM access under the script session boundary. HTML parsing, innerHTML and DOMParser produce attributes. On first cascade or earlier CSSOM access the operation parses the attribute once. Explicit attribute replacement/removal invalidates the component through ECS reconciliation. CSSOM mutation reflects from that block under a self-write guard. |
| Clone and rule declarations (D) | Clone copies Attributes and starts without the source InlineStyle. A fresh block derives from the clone’s attribute through the same operation. Rules retain winners/provenance on their parsed declaration block. Update the ECS clone/reconcile docs and tests and the VM host copy-set documentation in node_methods_extras.rs. Coordinate tree_clone.rs with L2 branch layout-change-on-blur-first-focus at 412ba0c7; reconcile its copy-set table on the actual landing base. |

At C1, inventory the 37 files containing Dimension::Percentage, including tests (verified 2026-11-23 at c3c78070 with the last command above). Audit pattern matches, borrowed and by-value uses, wildcard arms and comparisons as well. Exercise block, positioned, flex, grid, multicol and table layout with their actual percentage bases; exercise style, render, background, animation and transform readers. The transform_math resolve_origin_dim arm resolves mixed calc against the supplied reference size, with independent X/Y controls. Retire its TODO in C1. C2 owns Flex sizing and interpolation results. Each child, including T/D/H/S, checks existing and projected file sizes and places new tests in cohesive siblings. A required touch-time split gets a standalone predecessor and a PM scope decision before implementation.

## §2. Crossing invariants and acceptance at the landing head

| Crossing | Producer → consumer | Discriminating observation |
|---|---|---|
| Token identity × grammar | T → G | A number next to an ident after substitution remains two tokens. For `--x:2 0 10px; flex:var(--x)` inspect all three longhands (2, 0, 10px). A substituted negative literal basis is invalid at computed-value time. Direct parse rejects it at parse time. |
| Token identity × numeric precision | T → G → N | Direct and substituted percentages derived from the same source digits have identical stored values. N compares independently converted lexemes at a binary32 rounding boundary and overflow. |
| Inheritance × computed readers | T | Parent `--m:var(--n)` computes before child override. Check computed getter, inherited consumer and table anonymous-style writer. Distinguish guaranteed-invalid from an empty sequence during fallback, even when both public strings are empty. |
| Pending membership × cascade rank | T → D | T retains each originating entry’s position in the existing stable CascadePriority order through reparse, including same-rule declaration order. Apply expanded candidates at that position, including invalid-at-computed-time members, without ranking by hash iteration or presence of a key. D records and consumes that result for public order/provenance. Test forward/reverse source order, importance and competing rule specificity. Replacing the variable recomputes just the value associated with the winning record. |
| Membership × public identity | D | Border-spacing is one public property for enumeration, removal and provenance. Incomplete font/list-style membership cannot certify a complete group. Check overlapping groups, mixed priorities and an uncovered keyword in both orders. |
| Mutation × reflection × computed authority | D → S | D replaces the cssom storage serialization and attribute-only inline cascade import with a typed block. Guard self-writes, rehydrate on external replacement, initialize a clone from its copied attribute. A partial pending group on a clone matches fresh hydration. At S, set an unrelated inline property after storing grow 0.9999999 or length 0.1234567px. Inspect typed/computed precision after attribute reflection. |
| Grammar × evaluator | G → N | Test nonnegative literal rejection separately from number-typed calc(-1) accepted then clamped for a factor. N enumerates public parsing, overflow and top-level results for padding, font, vertical-align, grid, box model and media, including division by zero inside a larger sum. |
| Arithmetic × phase × animation | N → C1 → C2 | C1 consumes the tree simplified by N at computed time. With a 100px base, explicit basis and auto/width calc(10% + 2px) both use 12px. Exercise indefinite content probes, negative-result clamping after substitution of a base, and animation write/sampling across length and calc. |
| Typed selection × existing families | D → H | At D, old string handlers retain Box/Table/Multicol readback. At H a test handler distinguishes 0.9999999 from exactly 1. Check all-initial columns/column-rule under a single documented survivor rule. |
| Arithmetic × declared text × media keys | N → S | Reparse calc(infinity * 1px), calc(-infinity * 1px) and calc(NaN * 1px). Test stable ordering and matching on either side of two representable nearby media thresholds. S documents which contexts can share rounded text and which require lossless keys. |
| Typed omission × public rounding | H, S → F | grow/shrink/basis 0.9999999/1/auto reads as “1 auto”, exact 1/1/auto as “auto”. Reading the original twice preserves the distinction. Reparsing rounded output can produce exact 1. Check both public getters. |
| Grammar × omission × used basis | G, C2 → F | 0/1/0px → “0”, 1/1/0% → “0%”, 0/1/auto → “0 auto”. Reparse the resulting triple and check its layout effect. Check direction/wrap permutations and CSS-wide or pending-group readback through the coordinator. |
| Substitution × cascade collisions | T → D | For --n:2px, put margin-left:7px before/after margin:var(--n) 1px. Normal or equally important: later wins (1px/7px). With only one important, it wins in either order. Repeat with whole Flex var and direct grow, and cross-rule specificity. Keep the stable sorted entries through computation; D never introduces a second ordering structure. |
| Token identity × reflection and rounding | T → D → S | Preserve raw custom/pending sequences including numeric spelling through attribute reflection, external rehydration and clone. Ordinary typed public numeric output rounds; unresolved tokens do not. After unrelated mutation a source grow of 0.9999999 remains exact, while its clone from the rounded attribute computes 1, matching a fresh element from that attribute. |
| Strict grammar × evaluator | G′ → N | Unknown dimensions stay invalid before evaluation on all migrated paths. Run N’s ordinary/special numeric and media controls using the unified parse path. |
| Typed grammar × computed reader | G′ → C1 | Pair direct and declared mixed-length inputs on supported grid and origin routes. Preserve percentages through computation and supply the actual reference size at use. Enable a broader input category only with a working reader. |
| Computed representation × public output | C1 → S | Serialize the N-simplified computed payload via the resolved-style getter. A mixed percentage remains observable before a reference size is available; check computed text and used length independently. |
| Lexeme conversion × typed equality | N → H | Feed 0.9999999 and 1 through the actual parser/converter. Verify H sees two distinct factors before both serialize to 1. |
| Public formatting × existing fidelity slots | S | For #11-css-declared-value-serialization-fidelity verify numeric component output for declared versus computed callers; for #11-cssvalue-list-separator-fidelity check list separators survive numeric-leaf formatting. PM tracks residual nonnumeric/list defects. |
| Earlier Flex verdicts | D → H → S → F | PM 928 F1/F2: omit 0px by grammar without a WPT-only exception. F6: for flex and flex-flow a later var after initial reads as var(--x); the reverse reads initial after D. F7: justify the all-initial survivor from CSSOM equivalence and canonical ordering before adopting it. |

## §3. Spec coverage map

| Spec section | Step | Branch | Touch (compile/dispatch site) | Full enum? | User-input flow |
|---|---|---|---|---|---|
| css-values-5 §A Arbitrary Substitution Functions | replace substitution functions and form pending values | whole, embedded, CSS-wide result, invalid, member provenance | T reparse / D winner | supported var | yes |
| css-variables-1 §2 Defining Custom Properties: the --* family of properties | property definition table, Computed value | resolved sequence or guaranteed-invalid | T component storage/readers | unregistered properties | yes |
| css-variables-1 §3 Using Cascading Variables: the var() notation | replace a var() function | missing comma, empty or nonempty fallback | T substitution | yes | yes |
| css-syntax-3 §9 Serialization | token separators | adjacency and re-tokenization | T parser input / D reflection and rehydration | applicable token pairs | yes |
| css-cascade-4 §6.1 Cascade Sorting Order | compare rank | priority, specificity, appearance | D winner | crossing controls | yes |
| CSSOM §6.6 CSS Declaration Blocks | create, parse and reflect block | external attribute, clone, self write | D inline block | relevant paths | yes |
| CSSOM §6.6.1 The CSSStyleDeclaration Interface | getPropertyValue, priority, mutation | complete, partial, mixed winner | D coordinator and CSSOM | inline and rule reads | yes |
| CSSOM §6.7.2 Serializing CSS Values | list 1.1–1.3, omission step 2 and component output | preferred shorthand, equivalence, numeric precision | H / S / F | supported families | yes |
| css-flexbox-1 §5.3 Flex Direction and Wrap: the flex-flow shorthand | component grammar | direction, wrapping or both | F selection | yes | yes |
| css-flexbox-1 §7.1 The flex Shorthand | parse groups and fill defaults | none, factors, basis, bare zero | G parsing / F selection | supported basis | yes |
| css-values-4 §5 Numeric Data Types | convert value, validate dimension unit (§5.4) | literal range and dimension syntax | G units / N conversion | supported units | yes |
| css-values-4 §6 Distance Units: the <length> type | validate supported length units and unitless zero | dimension, zero or rejection | G / G′ strict parser | supported units | yes |
| css-values-4 §9.1 Numeric Functions | apply property range | computed if possible, otherwise used | G admission / C1–C2 resolution | numeric functions in subset | yes |
| css-values-4 §10.8 Syntax | calc-keyword leaves (§10.7.1–2) | e, pi, signed infinities, NaN | N evaluator | five keywords | yes |
| css-values-4 §10.9 Type Checking | typed result and §10.9.2 arithmetic | factor, length-percentage, signed zero, infinity, NaN | G parse / N evaluate | arithmetic subset | yes |
| css-values-4 §10.11 Computed Value | simplify then §10.12 range check | definite or percentage-dependent result | C1 consumes N simplification | supported arithmetic | yes |
| css-values-4 §10.14 Combination of Math Functions | combine | length and mixed-percentage interpolation | C2 animation | supported arithmetic | yes |
| css-flexbox-1 §7.2.3 The flex-basis property | determine used basis | auto, definite, indefinite content | C2 layout | supported basis | yes |
| css-flexbox-1 §9.2 Line Length Determination | determine flex base size (2) | computed content / content-dependent basis | C2 sizing and probe | applicable cases | yes |
| css-values-4 §10.10.1 Simplification | sum/product flatten and numeric product | unresolved leaves or canonical unit | N simplification, consumed by C1/S | arithmetic subset | yes |
| css-values-4 §10.13 Serialization | serialize a math function step 3, then tree algorithm where applicable | special numeric root, sorted sum/product, negate and invert | S public/media rendering | arithmetic subset | yes |

Reproduce the map with the repository webref tool. Use its body command with css-values-5 arbitrary-substitution, css-variables-1 defining-variables / using-variables, css-syntax-3 serialization, css-cascade-4 cascade-sort, cssom-1 css-declaration-block / serialize-a-css-value / serialize-a-css-component-value, css-flexbox-1 flex-property / flex-flow-property / flex-basis-property / line-sizing, and css-values-4 numeric-types / dimensions / numeric-function / calc-keywords / calc-type-checking / calc-computed-value / calc-range / calc-simplification / calc-serialize. Use heading css-values-4 10 for the math subsection titles. Also run webref body with cssom-1 the-cssstyledeclaration-interface and serialize-a-css-declaration-block, css-values-4 lengths, calc-syntax and combine-math. Verify titles with heading cssom-1 6, heading css-values-4 5, 6, 9 and 10, heading css-variables-1 2 and 3, and heading css-flexbox-1 5. Each child verifies the full normative branches at its own review.

## §4. Binding R2 finding ownership

IDs below are local to each frozen report. A second slice named in an action verifies a crossing on the first head that can expose it. Assignment records a review obligation. Closure requires the evidence in the child plan and implementation.

| Flex @ 9853348d | Owner | Obligation |
|---|---|---|
| F1, F7 | Umbrella | Split S, record scope and dependencies here, redirect the July PR 2 staging entry. |
| F2, M14 | S | Inventory every to_css_string use, including storage, resolved style, rule cssText, WPT harness, Multicol and media. List changed exact oracles and name a cohesive formatting module. |
| F3 | D → S | Separate authoritative storage from public formatting at D and exercise both precision regressions on the S head. |
| F4, F5 | H → F | Migrate serialize_omit_initial with Multicol Q3/Q4, docstring, tests and touched T rows. Report group (k) of #11-core-browser-parity-deviations, including Multicol plan rows, through PM. Derive one survivor rule from CSSOM §6.7.2 serialize a CSS value step 2 and justify remaining ties through canonical grammar order. F applies it to Flex. |
| F6, M8, M11 | S | Distinguish simplification from serialization, name the math-function algorithm for the special-value step, reproduce headings. |
| M9, M10 | F | Replace restated producer contracts with named dependencies. G/C1/C2 own literal admission and computed/used range evidence. |
| M12 | Umbrella | Submit the rule-block regrouping slot draft below to PM. |
| M13 | F | Record the WPT spelling difference and why CSS-wide omission can change the triple under #11-shorthand-omit-initial-csswide-omission. |

| Parser @ c2f27760 | Owner | Obligation |
|---|---|---|
| F1, F2 | Umbrella → G/G′/N/C1/C2 | Split grammar, arithmetic and computed representation. Carry each relevant crossing from §2 into its owner's own gate. |
| F3, F10, M20 | N | Inventory all evaluator callers with before/after and terminal conversion, including media and #11-media-css-values-fidelity. Specify all five keywords and include the three generated special spellings locally. |
| F4, F6, F7 | C1 → C2 | Define shared payload, ownership after loss of Copy, equality, writers including animation, interpolation and both layout readers. Inventory Dimension consumers including the transform TODO. Derive content once for definite/indefinite resolution. |
| F5, F8 | G | Follow T. Exercise var() via all three members, shared factor math typing and invalid-at-computed-time controls. |
| F9, M13, M14 | G | Repair moved docstrings, map each new parser function to its spec step. N owns evaluator docs, C1/C2 own resolver docs and verifies range timing. |
| F11, M12, M15 | G → G′ | Inventory unknown-unit fallback callers including float, transform and grid, migrate to the shared grammar without adapters. Enumerate unsupported sizing keywords and their disposition. |
| M16, M17, M18 | G/G′/N/C1/C2 | Each child names the relevant test modules and crates. G records pinned WPT grammar cases and results, C1/C2 computed/layout cases. Audit file sizes before placement, use sibling modules for new cohesive units. |
| M19 | Umbrella → G/G′/N/C1/C2 | Apply §1 landing and review order in each child. |

| Declaration order and token @ ab539f10 | Owner | Obligation |
|---|---|---|
| F1, F10 | D | Use T’s cascade result (PM 966) and own the public readback and pairwise activation matrix, including provenance versus mutation, clone and recomputation. |
| F2, F4, M13, M16 | T | Use one token predicate at declaration parse, resolution and merge. Delete expand_resolved_shorthand and its milestone note. Test quoted var text, case, re-tokenization, CSS-wide fallback and the receiving grammar. |
| F3, M9, M19, M24 | T | Define owned immutable sequences/invalid on the plugin tier and compute on the style component before inheritance. Inventory DOM computed-style readers and anonymous table-style writer, their observable outputs, crate tests and sibling test modules. |
| F5, M12 | D | Clone from the attribute. Revise derived-component documentation and locate hydration, self-write and external invalidation in ensure_inline_style, sync_to_attribute and attribute reconciliation. |
| F6 | Umbrella → H | This staging record precedes T. H updates the July trait example at the change of signature. |
| F7, F11 | H | Own the full signature migration, precision control, literal tests, Multicol dispositions and ratchet in one PR. D proves prior families still work at the string boundary. |
| F8, M22 | D | Limit provenance to complete public membership. Test incomplete mappings and uncovered background reverse order/keyword-alone. Account for each ledger member before asking PM to close #11-shorthand-direct-decl-cascade-order at D or transfer outstanding family coverage. |
| M14, M15, M17, M18, M20 | D | Refer to F through this umbrella. State the single-store same-block winner inference separately from normative cascade rank. Cite the pending-value definition and CSSOM §6.7.2 serialize a CSS value list steps 1.1–1.3. Name the style coordinator sibling test module. |
| M21 | T | Submit token-boundary, empty fallback and inherited-value defects and closure evidence for PM's ledger. |
| M23 | H → F | Report effects on #11-css-declared-value-serialization-fidelity and #11-shorthand-omit-initial-csswide-omission, both due for L1 re-evaluation December 15. |
| M25 | T, D | Both memos carry the post-push and merge-at-head protocol in §1. |

### Ledger drafts and review dates

- **Rule-block regrouping (proposed slot for PM registration).** CSSOM §6.6 serialize a CSS declaration block step 4.3 considers eligible shorthands, their priorities and representability. RuleStyleCssText in crates/dom/elidex-dom-api/src/cssom_sheet.rs currently emits each declaration via push_declaration. A rule with four equal margin longhands therefore lacks its eligible single margin output. Block-wide regrouping has a different ordering contract from single-property readback. Re-evaluate when the first new family handler lands or at the L1 ledger check on 2026-12-15. PM verifies the premise and registers the slot.
- **Mixed calc in translation functions (proposed distinct transform slot).** CSS Transforms 1 §7.1 permits length-percentage X/Y offsets. The translate function stores CssValue arguments and the transform_math resolve_translate_value path needs a reference-box-dependent calculation to support translateX(calc(10% + 2px)). That path is separate from the Dimension origin reader. Inventory this as a distinct feature pending approval; retain the current supported argument subset during G′ and never silently accept an unevaluable expression. Trigger at C1’s reference-box test or 2026-12-15. Origin resolution itself ships in C1.
- **M23, request through PM.** On 2026-12-15 reconcile #11-shorthand-direct-decl-cascade-order (D), #11-shorthand-omit-initial-csswide-omission (F), #11-css-declared-value-serialization-fidelity (S/F) and #11-css-wide-revert-keyword (H’s typed classification and existing value-kind behavior). Request PM extension to 2027-01-15 for outstanding items with D/H/S/F landing heads as earlier triggers. PM owns the ledger decision; do not close an item on a draft or assume an extension.

## §5. Review handoff

The July program remains the parent for subsequent shorthand families. The local frozen source plans are css-shorthand-flex-omit-initial at 9853348d (docs/plans/2026-11-shorthand-flex-omit-initial.md), css-flex-parser-prereq at c2f27760 (docs/plans/2026-11-flex-shorthand-parser-prerequisite.md), and css-shorthand-order-prereq at ab539f10 (docs/plans/2026-11-shorthand-declaration-order.md and docs/plans/2026-11-shorthand-token-substitution.md). Reconcile child drafts with the approved umbrella before requesting their individual full reviews.
