//! Build-time generator for the WHATWG HTML §13.5 named-entity table.
//!
//! This file is **not** part of the crate module tree — it is pulled in
//! only by the crate's `build.rs` via `#[path = "…/build_entities.rs"]`,
//! so its `phf_codegen` / `serde_json` use (build-dependencies, not
//! runtime dependencies) never reaches the library build. It reads the
//! vendored `assets/entities.json` (a pinned WHATWG spec snapshot) and
//! emits `$OUT_DIR/entities.rs` defining:
//!
//! - `NAMED_ENTITIES: phf::Map<&'static str, &'static str>` — key is the
//!   full identifier including the leading `&` (e.g. `"&amp;"`), value is
//!   the replacement character(s).
//! - `MAX_ENTITY_NAME_LEN: usize` — the longest key length in `char`s,
//!   used to bound the §13.2.5.73 longest-match probe.

use std::env;
use std::fs;
use std::io::Write;
use std::path::Path;

/// Generate `$OUT_DIR/entities.rs` from the vendored `entities.json`.
pub fn generate() {
    let manifest = env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR");
    let json_path = Path::new(&manifest).join("assets/entities.json");
    println!("cargo:rerun-if-changed=assets/entities.json");

    let raw = fs::read_to_string(&json_path).expect("read assets/entities.json");
    let parsed: serde_json::Value = serde_json::from_str(&raw).expect("parse entities.json");
    let obj = parsed
        .as_object()
        .expect("entities.json must be a JSON object");

    // Collect owned key/value-expr strings that outlive the phf builder.
    let mut keys: Vec<String> = Vec::with_capacity(obj.len());
    let mut value_exprs: Vec<String> = Vec::with_capacity(obj.len());
    let mut max_len = 0usize;
    for (name, val) in obj {
        let chars = val
            .get("characters")
            .and_then(|c| c.as_str())
            .expect("entity entry missing `characters`");
        max_len = max_len.max(name.chars().count());
        keys.push(name.clone());
        // `{:?}` on a &str yields a valid Rust string literal expression.
        value_exprs.push(format!("{chars:?}"));
    }

    let mut map = phf_codegen::Map::new();
    for (k, v) in keys.iter().zip(value_exprs.iter()) {
        map.entry(k.as_str(), v.as_str());
    }

    let out_dir = env::var("OUT_DIR").expect("OUT_DIR");
    let dest = Path::new(&out_dir).join("entities.rs");
    let mut f = fs::File::create(&dest).expect("create entities.rs");
    writeln!(
        f,
        "/// Longest named-entity identifier length in chars (incl. `&`)."
    )
    .unwrap();
    writeln!(f, "pub const MAX_ENTITY_NAME_LEN: usize = {max_len};").unwrap();
    writeln!(
        f,
        "/// WHATWG HTML §13.5 named character references (key incl. `&`)."
    )
    .unwrap();
    writeln!(
        f,
        "pub static NAMED_ENTITIES: phf::Map<&'static str, &'static str> = {};",
        map.build()
    )
    .unwrap();
}
