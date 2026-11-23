# Preserve component values through computed substitution

Draft T under PM 961, first implementation slice of the Flex omit-initial umbrella (review head 71f1daad). T can land on the existing declaration stores. It gives G and D a common receiving-property parser and computed custom-property values. Its public effects include corrected substitution and computed custom-property readback.

## §1. Substitution and reparse

| Source seam | Change |
|---|---|
| elidex-plugin, declared Var and ComputedStyle.custom_properties | Introduce an owned component-value sequence in a sibling values module. Each item retains token kind, spelling (including numeric lexeme), and nested blocks. Store either guaranteed-invalid or an immutable shared sequence per computed name. An element owns the map on its style component. Inheritance shares immutable values and never rebinds their references in the child's environment. |
| elidex-css, declaration.rs and parse_var_function | Replace raw.contains("var(") with recursive token recognition. Compare the decoded function identifier ASCII-insensitively. Capture an explicitly empty custom-property declaration as a valid empty sequence, replacing the current raw.is_empty() early return. Capture full fallbacks and use the same predicate for whole and embedded references, ignoring string/comment contents. |
| elidex-style, resolve/var_resolution | Replace has_var_reference and string rewriting with that predicate and substitution. Delete expand_resolved_shorthand and its M4-LFS-post note. Reparse substituted component sequences once through the receiving property's grammar and populated registry, including grid, border, background and border-spacing. The grammar returns actual members. D later selects independent winners. |
| elidex-style resolve/mod.rs:55, elidex-dom-api computed_style.rs:169, elidex-layout-table grid.rs:121 | Serialize computed sequences at both computed getter paths. A guaranteed-invalid and a genuinely empty sequence both serialize as the empty string, but only the former triggers fallback. Carry inherited computed values into anonymous table styles. |

Expose a single-value entry point around parse_property_value for the receiving property, with the original priority outside the value. Its parser consumes text: serialize components with CSS Syntax separators and retain original numeric spellings. Require exhaustive consumption. Tests re-tokenize the text and compare every token and nesting boundary. Reject malformed or grammar-invalid substituted values at computed time. A semicolon or priority inside the output cannot introduce another declaration.

**Capture before interpretation.** In elidex-css, parse_var_function must preserve the full fallback as component values starting immediately after the first argument comma. Represent no comma as absent, a bare comma as present with an empty sequence, and all other fallbacks as present with their full token sequence (including nested functions, commas and numeric lexemes). Whole and embedded var references consume this representation. Parsing the fallback as a generic length/value at declaration time currently changes number 0 to a length and collapses absent and empty. Migrate Var constructors, stored declaration serialization and resolution together in the token slice; preserve the comma when serializing an empty fallback. Public getter activation still belongs to the successor.

**Compute before inheritance.** Replace the raw-string inheritance contract of build_custom_properties. On each element seed inherited names with immutable computed component sequences or guaranteed-invalid state from the parent, apply local cascade winners, and compute the local dependency graph in that element’s environment, including forward references, cycles and fallbacks. Publish the resulting map on the existing ComputedStyle. Ordinary properties and inheriting children consume those computed sequences. An inherited value is terminal in the child’s graph. An empty valid sequence and guaranteed-invalid are distinct. Keep declared source on its declaration for public readback. Recompute parent and affected descendants through normal style invalidation when relevant declarations change.

## §2. Coupled invariants and landing gate

- **Boundaries × grammar.** With --n:2, margin:var(--n)px substitutes a number followed by an ident. Margin rejects the sequence. Conversely --n:2px supplies one dimension, so margin:var(--n) computes to four 2px members. Include a valid fraction, nested fallback, a string containing the literal characters var(, and a function nested inside a block.
- **Recognition × invalidation.** With an earlier valid margin, a later quoted "var(" followed by 2px is invalid at parse time and leaves the earlier margin intact. Embedded VAR(--n) and its lower-case form have the same effect. An otherwise valid winning reference which becomes invalid at computed time takes the property's unset behavior. D owns the independent-longhand source-order, importance and specificity matrix.
- **Fallback × receiving grammar.** With --missing absent, flex-shrink:var(--missing, 0) computes to 0; var(--missing, 0px) is invalid at computed value time and shrink takes its initial 1. Exercise the latter with an earlier valid shrink to detect revival. Verify the difference between absent and empty fallback using `margin: 3px var(--missing,)` (all margins 3px) and `margin: 3px var(--missing)` (invalid, margins initial). Check that a whole var with empty fallback survives parse/declared serialization/reparse with its comma.
- **Inheritance × lookup environment.** On a parent declare --n:2px and --m:var(--n). On its child set --n:7px and margin:var(--m); all margins must compute to 2px. Redeclare --m:var(--n) on that child as a same-element control: all margins become 7px. Reverse local declaration order and repeat; add a grandchild, an inherited guaranteed-invalid value with fallback and a parent update to exercise invalidation.
- **Limits × substitution.** Account for nesting, cycles, repeated references and total expanded output before splicing each sequence. Failed expansion and missing references without fallback use the existing invalid-at-computed-value handling. Confirm a cycle cannot loop or amplify tokens past the existing budget.
- **Declared × computed readback.** On the parent, read --m through getComputedStyle and expect 2px. The declared getter preserves var(--n). For var(--missing, initial), the receiving property takes the specified CSS-wide keyword. Check identical results on inline and parsed rules.
- **Budget × shared sequences.** Charge expanded occurrences before append, even for repeated references to a cached sequence. Preserve the resolver's depth 32 and output-size bound 65,536 bytes measured on the token-preserving serialization, including required separators. Exercise the exact limit and one over it, and dependencies in unused fallbacks when detecting cycles.

## §3. Spec coverage map

| Spec section | Step | Branch | Touch | Full enum? | User-input flow |
|---|---|---|---|---|---|
| css-values-5 §A Arbitrary Substitution Functions | replace substitution functions in a property, steps 1–4 | whole reference, embedded, sole CSS-wide keyword, invalid and limits | substitute_vars and computed reparse | supported var functions | yes |
| css-variables-1 §2 Defining Custom Properties: the --* family of properties | computed value and inheritance | compute local dependencies before inheriting | build_custom_properties | supported unregistered properties | yes |
| css-variables-1 §3 Using Cascading Variables: the var() notation | replace a var() function, steps 1–5 | absent, present-empty, nonempty component sequence | parse_var_function | ✓ | yes |
| css-syntax-3 §9 Serialization | token boundaries | round-trip token separators | parser input | applicable boundaries | yes |
| css-variables-1 §2.2 Guaranteed-Invalid Values | computed serialization | invalid sentinel versus empty sequence | computed getters | yes | yes |

Reproduce with webref body queries for css-values-5 arbitrary-substitution; css-variables-1 defining-variables, guaranteed-invalid and using-variables; and css-syntax-3 serialization. The grammar oracle is the receiving property's existing registered parser. [CSS Values substitution](https://drafts.csswg.org/css-values-5/#arbitrary-substitution).

### §3.1 User-input touch audit

Author CSS controls source and replacement tokens. A quoted var spelling remains a string token. A replacement cannot inject a declaration separator or priority into the caller. Test malformed input, nested functions, empty fallback, a cycle and a successful parse immediately after a rejected declaration.

## §4. Review and verification

Use sibling test modules named substitution_tokens in elidex-css declaration tests and elidex-style var_resolution, with owned-token tests alongside the new elidex-plugin values module. Exercise DOM computed getter and anonymous table inheritance in elidex-dom-api and elidex-layout-table. Run all-features crate tests for those five crates. Give PM ledger evidence for the fabricated dimension, lost empty fallback and child rebinding defects. The declaration-order slot stays with D.

Rebase onto the approved umbrella landing head and check the slice against its verdict before requesting T's separate full plan review. Implementation follows a passing verdict. Run repository CI, obtain PM push go, use single-pass post-push Codex review and merge only on PM's explicit merge-at-head instruction.
