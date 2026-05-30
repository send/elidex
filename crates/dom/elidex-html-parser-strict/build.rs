//! Build script: generate the WHATWG HTML §13.5 named-entity table from
//! the vendored `assets/entities.json` into `$OUT_DIR/entities.rs`.
//!
//! The generation logic lives in `src/tokenizer/build_entities.rs`, pulled
//! in here via `#[path]` so it is compiled only by the build script (it
//! depends on `phf_codegen` / `serde_json`, which are build-dependencies).

#[path = "src/tokenizer/build_entities.rs"]
mod build_entities;

fn main() {
    build_entities::generate();
}
