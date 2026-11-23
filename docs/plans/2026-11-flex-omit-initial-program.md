# Flex omit-initial program

**Decision requested.** Approve the following eight implementation PRs and their boundaries under PM 959. The end product is a declared-value getter for `flex` and `flex-flow` that selects a meaning-preserving shorthand from the actual winning typed declarations. An unrelated inline mutation must preserve those declarations' computed effect. This umbrella is the scope record for Flex under the July shorthand handler-dispatch program. PM 928/929 established the original prerequisites. The three November 23 binding reviews at 9853348d, c2f27760 and ab539f10 reopened the plans before implementation.

## §1. Landing order and ownership

The table gives the proposed landing order. The predecessor column lists direct technical dependencies. Slices with no path between them can be designed independently. Land this documentation and its July staging pointer before T. Approve this umbrella first, then require a separate full five-axis /elidex-plan-review on each slice's own memo and head before implementation. The three reviewed heads are source material for those rewrites.

| Order / PR | Requires | Deliverable and boundary |
|---|---|---|
| 1 T — token substitution | — | One component-value substitution and receiving-property reparse path on existing stores. Preserve token boundaries and lexemes, distinguish absent/empty/nonempty fallbacks, handle cycles and limits, compute before inheritance. Own detection, computed custom-property readers and removal of guessed shorthand expansion. Exercise a winning shorthand with no independent longhand. |
| 2 G — Flex grammar | T | Share factor and basis parsing across shorthand, built-in longhand and registered handler. Move the existing calc grammar from elidex-css to the shared plugin tier with its clients. Resolve bare zero, group order, defaults, number-typed calc factors, length/percentage basis and rejection of negative literals and unknown units. Substituted declarations pass the same grammar and three-longhand acceptance as direct input. |
| 3 N — numeric conversion and arithmetic | G | One original-lexeme conversion and typed calc evaluator for every consumer, including media. Specify storage bounds, signed zero, all five calc keywords, IEEE intermediate results and each caller's top-level range policy. Own changes in accepted numeric inputs and arithmetic for existing non-Flex callers. |
| 4 C — computed length-percentage | N | Introduce one computed payload for Dimension and FlexBasis, preserve unresolved percentages, move both cascade writers and animation apply/interpolation to its conversion. Layout consumes the resolution result for both sizing and its content probe. Cover width-derived auto and explicit basis. |
| 5 D — declaration order | T | Activate the authoritative typed per-longhand winner and provenance together in rule and inline stores. Both getters/priority, enumeration, inline set/remove/cssText, reflection, hydration, clone, attribute invalidation and computed cascade switch together. D owns shorthand versus longhand precedence. Existing family handlers receive strings through one coordinator conversion. |
| 6 H — typed shorthand handler contract | D | Replace the string-pair CssPropertyHandler serialization signature with ordered typed pairs for every handler and coordinator entry point at once. Migrate Box, Table, Multicol, helpers and serialize_parsed. Compare owner-provided typed initials before formatting. Own the common survivor rule and its Multicol consequences. |
| 7 S — shared numeric and calculation serialization | H, N | Separate public decimal formatting from typed storage and media breakpoint identity. One arithmetic-tree serializer supplies canonical operator and child order with explicit leaf context. Cover every to_css_string caller, simplification, finite roundoff and non-finite spellings. H precedes public rounding so existing handlers already receive typed values. |
| 8 F — Flex omission | G, C, S | Add FlexHandler selection for both shorthands on the existing coordinator path. Reuse H for typed dispatch and S only after selection. Preserve declaration readback, round-trip at exposed precision and the distinction between zero length and zero percentage. D and H reach F through S. |

The slice author attaches its local test matrix, source inventory and named spec steps to its own memo. The PM receives a scope question whenever a proposed finding adds a crate, consumer group or invariant axis. The author records the proposed owner and waits for a decision before adding that work. For each implementation head use the lane's single-pass post-push Codex review, PM verdict and explicit merge-at-head instruction.

## §2. Crossing invariants and acceptance at the landing head

| Crossing | Producer → consumer | Discriminating observation |
|---|---|---|
| Token identity × grammar | T → G | A number next to an ident after substitution remains two tokens. For `--x:2 0 10px; flex:var(--x)` inspect all three longhands (2, 0, 10px). A substituted negative literal basis is invalid at computed-value time. Direct parse rejects it at parse time. |
| Token identity × numeric precision | T → G → N | Direct and substituted percentages derived from the same source digits have identical stored values. N compares independently converted lexemes at a binary32 rounding boundary and overflow. |
| Inheritance × computed readers | T | Parent `--m:var(--n)` computes before child override. Check computed getter, inherited consumer and table reader. Distinguish guaranteed-invalid from an empty sequence during fallback, even when both public strings are empty. |
| Pending membership × cascade rank | T → D | D cascades each member then extracts the winning substituted value. Test forward/reverse source order, importance and competing rule specificity. Replacing the variable recomputes just the value associated with the winning record. |
| Membership × public identity | D | Border-spacing is one public property for enumeration, removal and provenance. Incomplete font/list-style membership cannot certify a complete group. Check overlapping groups, mixed priorities and an uncovered keyword in both orders. |
| Mutation × reflection × computed authority | D → S | D replaces the cssom storage serialization and attribute-only inline cascade import with a typed block. Guard self-writes, rehydrate on external replacement, initialize a clone from its copied attribute. A partial pending group on a clone matches fresh hydration. At S, set an unrelated inline property after storing grow 0.9999999 or length 0.1234567px. Inspect typed/computed precision after attribute reflection. |
| Grammar × evaluator | G → N | Test nonnegative literal rejection separately from number-typed calc(-1) accepted then clamped for a factor. N enumerates public parsing, overflow and top-level results for padding, font, vertical-align, grid, box model and media, including division by zero inside a larger sum. |
| Arithmetic × phase × animation | N → C | Simplify what is known at computed time. With a 100px base, explicit basis and auto/width calc(10% + 2px) both use 12px. Exercise indefinite content probes, negative-result clamping after substitution of a base, and animation write/sampling across length and calc. |
| Typed selection × existing families | D → H | At D, old string handlers retain Box/Table/Multicol readback. At H a test handler distinguishes 0.9999999 from exactly 1. Check all-initial columns/column-rule under a single documented survivor rule. |
| Arithmetic × declared text × media keys | N → S | Reparse calc(infinity * 1px), calc(-infinity * 1px) and calc(NaN * 1px). Test stable ordering and matching on either side of two representable nearby media thresholds. S documents which contexts can share rounded text and which require lossless keys. |
| Typed omission × public rounding | H, S → F | grow/shrink/basis 0.9999999/1/auto reads as “1 auto”, exact 1/1/auto as “auto”. Reading the original twice preserves the distinction. Reparsing rounded output can produce exact 1. Check both public getters. |
| Grammar × omission × used basis | G, C → F | 0/1/0px → “0”, 1/1/0% → “0%”, 0/1/auto → “0 auto”. Reparse the resulting triple and check its layout effect. Check direction/wrap permutations and CSS-wide or pending-group readback through the coordinator. |

## §3. Spec coverage map

| Spec section | Step | Branch | Touch (compile/dispatch site) | Full enum? | User-input flow |
|---|---|---|---|---|---|
| css-values-5 §A Arbitrary Substitution Functions | replace substitution functions and form pending values | whole, embedded, CSS-wide result, invalid, member provenance | T reparse / D winner | supported var | yes |
| css-variables-1 §2 Defining Custom Properties: the --* family of properties | compute before inheritance | resolved sequence or guaranteed-invalid | T component storage/readers | unregistered properties | yes |
| css-variables-1 §3 Using Cascading Variables | replace a var() function | missing comma, empty or nonempty fallback | T substitution | yes | yes |
| css-syntax-3 §9 Serialization | token separators | adjacency and re-tokenization | T parser input | applicable token pairs | yes |
| css-cascade-4 §6.1 Cascade Sorting Order | compare rank | priority, specificity, appearance | D winner | crossing controls | yes |
| CSSOM §6.6 CSS Declaration Blocks | create, parse and reflect block | external attribute, clone, self write | D inline block | relevant paths | yes |
| CSSOM §6.6.1 The CSSStyleDeclaration Interface | getPropertyValue, priority, mutation | complete, partial, mixed winner | D coordinator and CSSOM | inline and rule reads | yes |
| CSSOM §6.7.2 Serializing CSS Values | list 1.1–1.3, omission step 2 and component output | preferred shorthand, equivalence, numeric precision | H / S / F | supported families | yes |
| css-flexbox-1 §5.3 Flex Direction and Wrap | component grammar | direction, wrapping or both | F selection | yes | yes |
| css-flexbox-1 §7.1 The flex Shorthand | parse groups and fill defaults | none, factors, basis, bare zero | G parsing / F selection | supported basis | yes |
| css-values-4 §5 Numeric Data Types | convert value, validate dimension unit (§5.4) | literal range, length or unknown unit | G units / N conversion | supported units | yes |
| css-values-4 §9.1 Numeric Functions | apply property range | computed if possible, otherwise used | G admission / C resolution | numeric functions in subset | yes |
| css-values-4 §10.8 Syntax | calc-keyword leaves (§10.7.1–2) | e, pi, signed infinities, NaN | N evaluator | five keywords | yes |
| css-values-4 §10.9 Type Checking | typed result and §10.9.2 arithmetic | factor, length-percentage, signed zero, infinity, NaN | G parse / N evaluate | arithmetic subset | yes |
| css-values-4 §10.11 Computed Value | simplify then §10.12 range check | definite or percentage-dependent result | C computed value | supported arithmetic | yes |
| css-values-4 §10.14 Combination of Math Functions | combine | length and mixed-percentage interpolation | C animation | supported arithmetic | yes |
| css-flexbox-1 §7.2.3 The flex-basis property | determine used basis | auto, definite, indefinite content | C layout | supported basis | yes |
| css-flexbox-1 §9.2 Line Length Determination | determine flex base size (2) | computed content / content-dependent basis | C sizing and probe | applicable cases | yes |
| css-values-4 §10.10.1 Simplification | sum/product flatten and numeric product | unresolved leaves or canonical unit | S tree projection | arithmetic subset | yes |
| css-values-4 §10.13 Serialization | serialize a math function step 3, then tree algorithm where applicable | special numeric root, sorted sum/product, negate and invert | S public/media rendering | arithmetic subset | yes |

Reproduce the map with the repository webref tool. Use its body command with css-values-5 arbitrary-substitution, css-variables-1 defining-variables / using-variables, css-syntax-3 serialization, css-cascade-4 cascade-sort, cssom-1 css-declaration-block / serialize-a-css-value / serialize-a-css-component-value, css-flexbox-1 flex-property / flex-flow-property / flex-basis-property / line-sizing, and css-values-4 numeric-types / dimensions / numeric-function / calc-keywords / calc-type-checking / calc-computed-value / calc-range / calc-simplification / calc-serialize. Use heading css-values-4 10 for the math subsection titles. Each child verifies the full normative branches at its own review.

## §4. Binding R2 finding ownership

IDs below are local to each frozen report. A second slice named in an action verifies a crossing on the first head that can expose it. Assignment records a review obligation. Closure requires the evidence in the child plan and implementation.

| Flex @ 9853348d | Owner | Obligation |
|---|---|---|
| F1, F7 | Umbrella | Split S, record scope and dependencies here, redirect the July PR 2 staging entry. |
| F2, M14 | S | Inventory every to_css_string use, including storage, resolved style, rule cssText, WPT harness, Multicol and media. List changed exact oracles and name a cohesive formatting module. |
| F3 | D → S | Separate authoritative storage from public formatting at D and exercise both precision regressions on the S head. |
| F4, F5 | H → F | Migrate serialize_omit_initial with Multicol Q3/Q4, docstring, tests and touched T rows. Report group (k), including Multicol plan rows, through PM. Derive one survivor rule from CSSOM step 2 and justify remaining ties through canonical grammar order. F applies it to Flex. |
| F6, M8, M11 | S | Distinguish simplification from serialization, name the math-function algorithm for the special-value step, reproduce headings. |
| M9, M10 | F | Replace restated producer contracts with named dependencies. G/C own literal admission and computed/used range evidence. |
| M12, M13 | F | Give PM a slot-fit disposition for rule-block regrouping, record WPT spelling divergence and why dropping a CSS-wide value can change the triple. Update the family omission slot through PM. |

| Parser @ c2f27760 | Owner | Obligation |
|---|---|---|
| F1, F2 | Umbrella → G/N/C | Split grammar, arithmetic and computed representation. Carry each relevant crossing from §2 into its owner's own gate. |
| F3, F10, M20 | N | Inventory all evaluator callers with before/after and terminal conversion, including media and its fidelity slot. Specify all five keywords and include the three generated special spellings locally. |
| F4, F6, F7 | C | Define shared payload, ownership after loss of Copy, equality, writers including animation, interpolation and both layout readers. Inventory Dimension consumers including the transform TODO. Derive content once for definite/indefinite resolution. |
| F5, F8 | G | Follow T. Exercise var() via all three members, shared factor math typing and invalid-at-computed-time controls. |
| F9, M13, M14 | G | Repair moved docstrings, map each new parser function to its spec step. N owns evaluator docs, C owns resolver docs and verifies range timing. |
| F11, M12, M15 | G | Inventory unknown-unit fallback callers including float, transform and grid, migrate to the shared grammar without adapters. Enumerate unsupported sizing keywords and their disposition. |
| M16, M17, M18 | G/N/C | Each child names the relevant test modules and crates. G records pinned WPT grammar cases and results, C computed/layout cases. Audit file sizes before placement, use sibling modules for new cohesive units. |
| M19 | Umbrella → G/N/C | Apply §1 landing and review order in each child. |

| Declaration order and token @ ab539f10 | Owner | Obligation |
|---|---|---|
| F1, F10 | D | Own the four precedence directions and pairwise activation matrix, including provenance versus mutation, clone and recomputation. |
| F2, F4, M13, M16 | T | Use one token predicate at declaration parse, resolution and merge. Delete expand_resolved_shorthand and its milestone note. Test quoted var text, case, re-tokenization, CSS-wide fallback and the receiving grammar. |
| F3, M9, M19, M24 | T | Define owned immutable sequences/invalid on the plugin tier and compute on the style component before inheritance. Inventory DOM computed-style and table readers, their observable outputs, crate tests and sibling test modules. |
| F5, M12 | D | Clone from the attribute. Revise derived-component documentation and locate hydration, self-write and external invalidation in ensure_inline_style, sync_to_attribute and attribute reconciliation. |
| F6 | Umbrella → H | This staging record precedes T. H updates the July trait example at the change of signature. |
| F7, F11 | H | Own the full signature migration, precision control, literal tests, Multicol dispositions and ratchet in one PR. D proves prior families still work at the string boundary. |
| F8, M22 | D | Limit provenance to complete public membership. Test incomplete mappings and uncovered background reverse order/keyword-alone. Account for each ledger member before asking PM to close the order slot at D or transfer outstanding family coverage. |
| M14, M15, M17, M18, M20 | D | Refer to F through this umbrella. State the single-store same-block winner inference separately from normative cascade rank. Cite the pending-value definition and CSSOM list steps 1.1–1.3. Name the style coordinator sibling test module. |
| M21 | T | Submit token-boundary, empty fallback and inherited-value defects and closure evidence for PM's ledger. |
| M23 | H → F | Report effects on declared-value fidelity and CSS-wide omission slots, both due for L1 re-evaluation December 15. |
| M25 | T, D | Both memos carry the post-push and merge-at-head protocol in §1. |

### Scope questions for the umbrella verdict

G owns the strict shared unit parser. Parser R2 F11 also identifies the separately implemented grid parser and direct float/transform consumers. We propose migrating these siblings where they share that grammar, subject to PM confirmation of those consumer groups. C owns the Dimension conversion, Flex layout and animation. We propose migrating every reader needed for that type to compile and consume it correctly. A distinct transform feature behind the existing TODO would require a separate scope decision. F owns reporting the rule-block regrouping gap to PM for slot-fit review. Implementing rule-block regrouping would extend the program.

## §5. Review handoff

The July program remains the parent for subsequent shorthand families. The frozen Flex serializer, parser and order/token plans are available on css-shorthand-flex-omit-initial, css-flex-parser-prereq and css-shorthand-order-prereq respectively. Replace their old staging when writing the reviewed children. Review the umbrella boundary before submitting any child for review. An umbrella verdict establishes ownership and landing constraints. Each child still needs its own implementation clearance.
