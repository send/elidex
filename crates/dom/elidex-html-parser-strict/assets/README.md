# Vendored WHATWG named-character-reference table

`entities.json` is a pinned snapshot of the WHATWG HTML "Named character
references" data (HTML §13.5), the canonical source for the named-entity
table the tokenizer resolves in the named-character-reference state
(§13.2.5.73).

- **Source**: <https://html.spec.whatwg.org/entities.json>
- **Format**: object keyed by the full identifier including the leading
  `&` (e.g. `"&amp;"`), value `{"codepoints":[…],"characters":"…"}`.
- **Consumed by**: `build.rs` → `src/tokenizer/build_entities.rs`, which
  generates a `phf::Map<&str, &str>` (name → replacement characters) into
  `$OUT_DIR/entities.rs` at build time.
- **Why vendored**: reproducible / offline builds (A2 plan decision D-c —
  no CDN fetch at build time).

To refresh: re-download from the URL above and rebuild; the codegen and
longest-match probe adapt to the new contents automatically.
