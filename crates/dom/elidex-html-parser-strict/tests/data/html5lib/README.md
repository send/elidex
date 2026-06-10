# Vendored html5lib-tests (tokenizer + tree construction)

The `tokenizer/*.test` JSON files and the `tree-construction/*.dat` files in
this directory are a vendored snapshot of the
[html5lib-tests](https://github.com/html5lib/html5lib-tests) conformance
suite — language-agnostic test **vectors** (not a parser library):

- `tokenizer/*.test` describe the expected WHATWG HTML §13.2.5 token output
  for given inputs (used by `src/tokenizer/tests_html5lib.rs`).
- `tree-construction/*.dat` (`tests1`, `tests2`, `doctype01`,
  `tests_innerHTML_1`) describe the expected §13.2.6 DOM tree (`#document`) and
  parse errors (`#errors`) for given inputs (used by
  `src/tree_builder/tests_html5lib_tree.rs`). `tests_innerHTML_1` is the
  §13.4 fragment (`innerHTML`) suite: each case carries a `#document-fragment`
  context element and is driven through `parse_fragment_strict`.

Details:

- **Source**: <https://github.com/html5lib/html5lib-tests> (`tokenizer/` and
  `tree-construction/`).
- **License**: MIT. The upstream license text is vendored alongside the data
  at [`LICENSE`](./LICENSE) (Copyright © 2006-2013 James Graham, Geoffrey
  Sneddon, and other contributors) — its terms require the copyright notice
  and permission notice to ship with any redistribution of these files.
- **Why vendored**: build/test reproducibility and offline runs (the Phase A
  plan declines CDN fetch at build/test time, mirroring the entity-table
  decision D-c).
- **Tree-construction subset**: only the general suites are vendored;
  foreign-content (SVG/MathML, deferred to A5) and adoption-agency suites are
  excluded as out of scope for the strict (no-recovery) builder. Strict mode
  rejects on the first parse error, so a `.dat` case with a non-empty
  `#errors` list is asserted to abort, and a case with no errors must
  reproduce `#document` exactly. `#document-fragment` cases run through the
  strict §13.4 fragment parser. The harness skips (and counts) cases the
  strict path does not cover: `#script-off`, foreign content (`<svg>` /
  `<math>` or a namespaced fragment context), and the rare case whose vendored
  `#errors` / `#document` predates a spec change elidex implements (e.g. the
  customizable-`<select>` removal of the "in select" mode) — surfaced with its
  divergence reason rather than silently dropped.

These are test inputs only; no html5lib code is linked into the crate. The
runtime parser is fully self-implemented (`src/tokenizer/` + `src/tree_builder/`),
depending only on `elidex-ecs` + `phf`.
