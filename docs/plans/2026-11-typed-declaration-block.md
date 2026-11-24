# Atomic typed CSS declaration block activation

D is row 8 of the [Flex omit-initial program](2026-11-flex-omit-initial-program.md). Planning base 9a88e160 includes the C2 plan (#755). Before implementation identify T’s **implementation** merge and verify its plugin comparator, pending member identity and rank at the actual integration head. Consume the [G′b.6 C-13 handoff](2026-11-length-reader-closeout.md) at activation. D activates rule construction, inline materialization and mutation, reflection, CSSOM reads and cascade together. H later migrates handler signatures; S owns numeric formatting and F owns Flex omission.

## §1. Change and acceptance

Define the declaration/winner, pending token and group provenance payloads in elidex-plugin for both rule and entity-owned inline blocks. Keep the sole registry-aware materializer and mutator in elidex-style. ensure_inline_style delegates to it, as does the style walk before collecting inline declarations. Construct rule winners at parse time so an unmatched or unattached rule can still be read. Use T’s comparator both for construction and mutation; cascade ranks these winners across blocks. Cover priority, enumeration, removal, partial groups, readback and reflection from the same data.

HTML parsing, DOMParser and innerHTML initially provide attributes. Guard self-writes; reconcile external set/remove and public EcsDom::clone_attributes (including replacement on a destination with an existing inline block). On clone copy attributes, then materialize from the copy. Test CSSOM-first and cascade-first hydration, guard/rehydrate, source precision versus fresh clone, anonymous and standalone rule reads. Keep the one string conversion until H. Reconcile the L2 tree_clone copy-set on the landing base and update node_methods_extras.rs host docs.

Correct lazy/string-storage comments in custom-elements/created_element.rs:20, html-parser/element_init.rs:29,189, style/cascade/mod.rs:373, style/lib.rs:77 and plugin/values.rs:577,810. Update the inline regrouping citation to the single PM-registered parent slot. Future writable rule.style is downstream. Retain PM 966’s existing correct order controls at every intermediate head.

## §2. Crossing checks

| Crossing | Producer → consumer | Discriminating observation |
|---|---|---|
| Pending membership × cascade rank | T → D | Build the per-member block winner with T’s comparator and reparse at its position. An invalid computed result stays with that winning declaration. Verify rule reads before matching and correct cascade after a variable change. |
| Membership × public identity | D | Border-spacing is one public property for enumeration, removal and provenance. Incomplete font/list-style membership cannot certify a complete group. Check overlapping groups, mixed priorities and an uncovered keyword in both orders. |
| Mutation × reflection × computed authority | D → S | D replaces the cssom storage serialization and attribute-only inline cascade import with a typed block. Guard self-writes, rehydrate on external replacement, initialize a clone from its copied attribute. A partial pending group on a clone matches fresh hydration. At S, set an unrelated inline property after storing grow 0.9999999 or length 0.1234567px. Inspect typed/computed precision after attribute reflection. |
| Typed selection × existing families | D → H | At D, old string handlers retain Box/Table/Multicol readback. At H a test handler distinguishes 0.9999999 from exactly 1. Check all-initial columns/column-rule under a single documented survivor rule. |
| Substitution × cascade collisions | T → D | For --n:2px, put margin-left:7px before/after margin:var(--n) 1px. Normal or equally important: later wins (1px/7px). With only one important, it wins in either order. Repeat with whole Flex var and direct grow, and cross-rule specificity. Feed block winners to cross-block cascade and check the same order through CSSOM. |
| Token identity × reflection and rounding | T → D → S | Preserve raw custom/pending sequences including numeric spelling through attribute reflection, external rehydration and clone. Ordinary typed public numeric output rounds; unresolved tokens do not. After unrelated mutation a source grow of 0.9999999 remains exact, while its clone from the rounded attribute computes 1, matching a fresh element from that attribute. |
| Earlier Flex verdicts | D → H → S → F | PM 928 F1/F2: omit 0px by grammar without a WPT-only exception. F6: for flex and flex-flow a later var after initial reads as var(--x); the reverse reads initial after D. F7: justify the all-initial survivor from CSSOM equivalence and canonical ordering before adopting it. |

## §3. Spec coverage map

| Spec section | Step | Branch | Touch (compile/dispatch site) | Full enum? | User-input flow |
|---|---|---|---|---|---|
| css-values-5 §A Arbitrary Substitution Functions | replace substitution functions and form pending values | whole, embedded, CSS-wide result, invalid, member provenance | T reparse / D winner | supported var | yes |
| css-syntax-3 §9 Serialization | token separators | adjacency and re-tokenization | T parser input / D reflection and rehydration | applicable token pairs | yes |
| css-cascade-4 §6.1 Cascade Sorting Order | compare rank | priority, specificity, appearance | D winner | crossing controls | yes |
| CSSOM §6.6 CSS Declaration Blocks | create, parse and reflect block | external attribute, clone, self write | D inline block | relevant paths | yes |
| CSSOM §6.6.1 The CSSStyleDeclaration Interface | getPropertyValue, priority, mutation | complete, partial, mixed winner | D coordinator and CSSOM | inline and rule reads | yes |
| WHATWG DOM §4.4 Interface Node | clone a node, copy element attributes | copy to fresh element | tree_clone and CSSOM declaration-block creation | elements | yes |

The breadth recommendation spans two public block owners and their common consumers. Keep activation in one implementation PR: switching either reader alone would expose different winners to CSSOM and cascade. T lands the comparator first. A touch-time file split can precede activation while preserving the existing production paths.

## Binding R3 acceptance

**C-1, C-13.** elidex-css builds rule blocks at parse time using the plugin comparator, including rules with no matching element. The style materializer uses that comparator for inline blocks. Retire T’s within-block replay when D activates; cross-block cascade takes one winning candidate per member per block. Test rule getter before cascade, inline mutation, and cross-block importance and specificity.

**C-8.** Reconcile every family/member and exception in #11-shorthand-direct-decl-cascade-order with a direct winner, provenance and priority test or a PM-approved remaining home. Give PM this accounting before requesting closure.

**C-10.** Complete the documentation audit with attribute_reconcile.rs:37–43, inline_style.rs:3–5 and 28–33, declaration/cssom.rs:16–34, dom-api/style.rs:4–10, 36–42 and 104–107, and tree_clone.rs:25. Check the comments against the actual read/mutation and invalidation paths.

**C-11, C-12.** In the mutable style phase, hydrate only when the component is absent; in the script session, the CSSOM entry delegates reads and mutations to the same style operation. ECS reconciliation, tree_clone and clone_attributes only invalidate derived state. Verify that registry access and reconstruction occur at the next style or CSSOM entry.


## Implementation acceptance and evidence

**C-13 production matrix (F8).** Use an unmatched stylesheet rule and an element initialized from the same inline attribute. Read each block before any style walk; then match the rule or attach the element and query its computed style. Repeat on fresh elements in cascade-first order. Use margin shorthand P = margin:var(--n) 1px with --n:2px and longhand L = margin-left:7px. Require these results for both block owners. Read declared values without substituting custom properties. The priority getter for the winning left member returns important exactly when the winner is important.

| Within-block source order and priority | Winning left member and declared read | Computed left |
|---|---|---|
| L then P, same priority (both normal or both important) | P; left getter empty, intact shorthand getter preserves pending value | 1px |
| P then L, same priority | L; left getter 7px, partial pending shorthand getter empty | 7px |
| P important, L normal, each source order | P; intact pending group | 1px |
| L important, P normal, each source order | L; direct left, remaining three pending members | 7px |

Repeat with flex:var(--f) (--f:2 1 0px) and flex-grow:3; the corresponding computed winners are 2 and 3. Change --f to an invalid substitution: winning pending members take unset, a winning direct grow remains 3. Use independent selectors to separate same-block position from cross-block specificity, then test normal inline versus important rule and equally important inline versus rule. Record priority, source position and group identity through the selected candidate. Compare shorthand and longhand getters, enumeration, cssText and computed winners separately. Keep pre-existing output modes for the rule’s source text and declared getters.

| Other branch (F8) | Planning-base production path → authoritative reader | Discriminating fixture |
|---|---|---|
| Unmatched/anonymous rule | elidex-css rule parser → plugin block → dom-api/cssom_sheet.rs RuleStyleGetPropertyValue, priority, length/item; later matching → cascade | Run C-13 before a matching element exists; enumerate one entry per public longhand and verify winner metadata survives rule matching. |
| Parsed inline versus CSSOM mutation | declaration/cssom.rs parse_inline_style and dom-api/style.rs ensure_inline_style → style-owned materializer/mutator → inline getters and cascade | After parsing an important left, setProperty replaces it with normal 9px; priority clears and computed left becomes 9px. Invalid value or invalid priority leaves it intact. removeProperty returns the previous declared value and removes it. Test whole cssText replacement, empty-value removal, enumeration and grouped shorthand restoration after assigning every member. |
| Cascade-first and CSSOM-first materialization | Mutable style-walk entry / DOM API → same entity InlineStyle; get_inline_declarations consumes current winners | HTML parser, DOMParser and innerHTML supply equivalent attributes. Exercise registry-backed transform plus custom and ordinary declarations on each order. Read twice; compare the same component and computed values. |
| Self-reflection versus external replacement/removal | sync_to_attribute → attribute_reconcile; external attribute invalidation → next style or CSSOM entry | Preserve unrounded original and raw pending/custom tokens after unrelated set. Replace the attribute, then remove it; a retained style wrapper and cascade see the new block then an empty block. Confirm existing attribute change notifications survive the self-write guard. |
| Fresh clone versus replacement destination | tree_clone / EcsDom::clone_attributes → copied attributes → materialization | Clone partially overwritten pending groups; compare fresh hydration from identical bytes. Seed a destination with a different authoritative block and verify stale winners disappear, including when the source lacks style. Mutating the clone leaves source typed values and priorities intact. |
| Shared handler bridge | Block member selection → single D string conversion → existing shorthand handlers | Compare rule and inline Box/Table/Multicol getters. Border-spacing keeps its single public identity; mixed priorities and partial memberships return empty shorthand. Carry exact typed initial comparison to H. |

**M1.** Put comparator and provenance tests with plugin, parse/expansion with css, cross-block winners with style/cascade, invalidation and clone with ecs/dom, declared read/mutation with dom-api, and actual rule.style/element.style and computed reads with the existing JS CSSOM tests. Run affected all-features suites for elidex-plugin, elidex-css, elidex-style, elidex-ecs, elidex-dom-api, elidex-js, the HTML parser, affected property families and elidex-wpt. At the planning base declaration.rs, dom-api/style.rs and dom-api/cssom_sheet.rs are 836, 540 and 873 lines; recount all touched modules and apply the standalone split rule. At WPT revision e00a7b14e8e3d9c04933e29ed247baf76c60706e select applicable assertions from css/cssom/cssom-setProperty-shorthand.html, cssstyledeclaration-csstext-important.html and CSSStyleDeclaration-indexed-properties.html (all in css/cssom). Record test path, selected assertion, runner, revision, implementation SHA and outcome; identify adapted tests separately. Run umbrella §3 gates.

**M2.** For every row record actual implementation file, symbol, reviewed head and main merge SHA for producer, comparator, authoritative field, mutation/invalidation and both public and cascade readers. Trace rule and inline ingress, registry ownership, the CSSOM host wrappers, style walk and T substitution. Verify removal of the attribute-only cascade parse, string-as-authority storage and redundant within-block replay; enumerate surviving serialization calls and H’s one handler conversion. Reconcile the direct-declaration slot’s family list and inline regrouping reference with PM at D landing (December 15 backstop). Report actual D matrix evidence back to G′b.6 and the umbrella.
