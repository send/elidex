# Vendored html5lib-tests (tokenizer)

The `tokenizer/*.test` JSON files in this directory are a vendored snapshot
of the [html5lib-tests](https://github.com/html5lib/html5lib-tests)
tokenizer conformance suite — language-agnostic test **vectors** (not a
parser library) describing the expected WHATWG HTML §13.2.5 token output
for given inputs.

- **Source**: <https://github.com/html5lib/html5lib-tests> (`tokenizer/`)
- **License**: MIT (see the html5lib-tests `LICENSE`).
- **Why vendored**: build/test reproducibility and offline runs (the A2
  plan declines CDN fetch at build/test time, mirroring the entity-table
  decision D-c).
- **Used by**: `src/tokenizer/tests_html5lib.rs`. Strict mode rejects on
  the first parse error, so a test whose `errors` list is non-empty is
  asserted to abort; a test with no errors must reproduce `output` exactly.

These are test inputs only; no html5lib code is linked into the crate. The
runtime parser is fully self-implemented (`src/tokenizer/`), depending only
on `elidex-ecs` + `phf`.
