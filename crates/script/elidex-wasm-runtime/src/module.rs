//! Compiled module + engine-indep introspection.
//!
//! `WasmModule` wraps a compiled `wasmtime::Module` (tier-B `pub(crate)
//! inner`) and the original source bytes (`Arc<[u8]>`) so custom-section
//! lookup can be implemented engine-indep — `wasmtime::Module` does not
//! expose `custom_sections` on its public surface.
//!
//! Spec anchors:
//! - WASM JS API §5.1 Module static methods: `imports()`, `exports()`,
//!   `customSections(sectionName)`
//! - WASM JS API §5.1 `ImportExportKind` IDL enum (current spec lists 5
//!   variants: function / table / memory / global / tag — elidex MVP
//!   defers `tag` per Exception Handling host machinery 未実装,
//!   `#[non_exhaustive]` carries the additive room)

use std::sync::Arc;

/// A compiled WebAssembly module. Engine-bridge tier B — `inner` holds
/// the wasmtime artifact and `source_bytes` keeps the original bytes for
/// custom-section lookup.
#[derive(Clone)]
pub struct WasmModule {
    pub(crate) inner: wasmtime::Module,
    pub(crate) source_bytes: Arc<[u8]>,
}

impl std::fmt::Debug for WasmModule {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("WasmModule").finish_non_exhaustive()
    }
}

impl WasmModule {
    /// Per WASM JS API §5.1 `Module.imports()` static method — returns a
    /// list of engine-indep import descriptors. Imports of unsupported
    /// kinds (e.g. `tag` until Exception Handling lands) are skipped.
    pub fn imports(&self) -> Vec<ModuleImportDescriptor> {
        self.inner
            .imports()
            .filter_map(|imp| {
                Some(ModuleImportDescriptor {
                    module: imp.module().to_string(),
                    name: imp.name().to_string(),
                    kind: import_export_kind_from(&imp.ty())?,
                })
            })
            .collect()
    }

    /// Per WASM JS API §5.1 `Module.exports()` static method — returns a
    /// list of engine-indep export descriptors. Exports of unsupported
    /// kinds (e.g. `tag`) are skipped.
    pub fn exports(&self) -> Vec<ModuleExportDescriptor> {
        self.inner
            .exports()
            .filter_map(|exp| {
                Some(ModuleExportDescriptor {
                    name: exp.name().to_string(),
                    kind: import_export_kind_from(&exp.ty())?,
                })
            })
            .collect()
    }

    /// Per WASM JS API §5.1 `Module.customSections(sectionName)` static
    /// method — returns the payload bytes of every custom section whose
    /// name matches `name`. Order matches the module binary order
    /// (spec: "in their original order in the module"). A module may
    /// declare multiple custom sections with the same name, so the
    /// return type is a `Vec` of payloads, not `Option`.
    pub fn custom_sections(&self, name: &str) -> Vec<Vec<u8>> {
        custom_section_payloads(&self.source_bytes, name)
    }
}

/// Engine-indep import descriptor per WASM JS API §5.1.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ModuleImportDescriptor {
    pub module: String,
    pub name: String,
    pub kind: ImportExportKind,
}

/// Engine-indep export descriptor per WASM JS API §5.1.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ModuleExportDescriptor {
    pub name: String,
    pub kind: ImportExportKind,
}

/// Kind of imported / exported entity per WASM JS API §5.1
/// `ImportExportKind` IDL enum.
///
/// Spec lists 5 variants; elidex MVP exposes 4. `tag` is deferred per
/// the Exception Handling proposal host machinery being unimplemented
/// (tracked as `#11-wasm-exception-handling`). `#[non_exhaustive]` lets
/// the proposal land additively without breaking downstream `match`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum ImportExportKind {
    Function,
    Table,
    Memory,
    Global,
}

fn import_export_kind_from(ty: &wasmtime::ExternType) -> Option<ImportExportKind> {
    match ty {
        wasmtime::ExternType::Func(_) => Some(ImportExportKind::Function),
        wasmtime::ExternType::Table(_) => Some(ImportExportKind::Table),
        wasmtime::ExternType::Memory(_) => Some(ImportExportKind::Memory),
        wasmtime::ExternType::Global(_) => Some(ImportExportKind::Global),
        wasmtime::ExternType::Tag(_) => None,
    }
}

// ---------------------------------------------------------------------------
// Custom section walker (WASM binary format)
// ---------------------------------------------------------------------------

/// Walk the WASM binary section list and collect payloads of custom
/// sections (section ID 0) whose name matches `target_name`.
///
/// Format reference (WebAssembly Core Spec §5.1 / §5.5 / §5.6):
/// - Header: 8 bytes (`\0asm` + `\x01\0\0\0` version)
/// - Sections: section_id (u8) + payload_size (LEB128 u32) + payload
/// - Custom section payload: name_size (LEB128 u32) + name + data
fn custom_section_payloads(bytes: &[u8], target_name: &str) -> Vec<Vec<u8>> {
    let mut out = Vec::new();
    if bytes.len() < 8 || &bytes[0..4] != b"\0asm" {
        return out;
    }
    let mut pos = 8;
    while pos < bytes.len() {
        let id = bytes[pos];
        pos += 1;
        let Some((payload_size, lebsz)) = read_leb128_u32(&bytes[pos..]) else {
            return out;
        };
        pos += lebsz;
        let Some(payload_end) = pos.checked_add(payload_size as usize) else {
            return out;
        };
        if payload_end > bytes.len() {
            return out;
        }
        if id == 0 {
            let payload = &bytes[pos..payload_end];
            if let Some((name_size, name_lebsz)) = read_leb128_u32(payload) {
                let name_end = name_lebsz.saturating_add(name_size as usize);
                if name_end <= payload.len() {
                    let name = &payload[name_lebsz..name_end];
                    if name == target_name.as_bytes() {
                        out.push(payload[name_end..].to_vec());
                    }
                }
            }
        }
        pos = payload_end;
    }
    out
}

/// Decode an unsigned LEB128 (up to 5 bytes for u32) at the start of
/// `buf`. Returns `(value, bytes_consumed)` or `None` on overflow /
/// truncation.
fn read_leb128_u32(buf: &[u8]) -> Option<(u32, usize)> {
    let mut result: u32 = 0;
    let mut shift = 0_u32;
    for (i, &b) in buf.iter().enumerate().take(5) {
        let chunk = u32::from(b & 0x7f);
        result = result.checked_add(chunk.checked_shl(shift)?)?;
        if b & 0x80 == 0 {
            return Some((result, i + 1));
        }
        shift = shift.checked_add(7)?;
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    fn engine() -> wasmtime::Engine {
        wasmtime::Engine::default()
    }

    fn compile(wat: &str) -> WasmModule {
        let bytes = wat::parse_str(wat).unwrap();
        let inner = wasmtime::Module::new(&engine(), &bytes).unwrap();
        WasmModule {
            inner,
            source_bytes: Arc::from(bytes.into_boxed_slice()),
        }
    }

    #[test]
    fn imports_round_trip_function() {
        let m = compile(
            r#"(module
                (import "env" "host_fn" (func (param i32) (result i32)))
            )"#,
        );
        let imports = m.imports();
        assert_eq!(
            imports,
            vec![ModuleImportDescriptor {
                module: "env".to_string(),
                name: "host_fn".to_string(),
                kind: ImportExportKind::Function,
            }]
        );
    }

    #[test]
    fn imports_round_trip_memory_global_table() {
        let m = compile(
            r#"(module
                (import "env" "mem" (memory 1))
                (import "env" "g" (global i32))
                (import "env" "t" (table 1 funcref))
            )"#,
        );
        let imports = m.imports();
        let kinds: Vec<_> = imports.iter().map(|i| i.kind).collect();
        assert_eq!(
            kinds,
            vec![
                ImportExportKind::Memory,
                ImportExportKind::Global,
                ImportExportKind::Table,
            ]
        );
    }

    #[test]
    fn exports_round_trip_all_kinds() {
        let m = compile(
            r#"(module
                (func (export "f") (result i32) i32.const 0)
                (memory (export "mem") 1)
                (table (export "t") 1 funcref)
                (global (export "g") i32 (i32.const 0))
            )"#,
        );
        let exports = m.exports();
        let by_name: std::collections::BTreeMap<_, _> =
            exports.iter().map(|e| (e.name.as_str(), e.kind)).collect();
        assert_eq!(by_name["f"], ImportExportKind::Function);
        assert_eq!(by_name["mem"], ImportExportKind::Memory);
        assert_eq!(by_name["t"], ImportExportKind::Table);
        assert_eq!(by_name["g"], ImportExportKind::Global);
    }

    #[test]
    fn exports_empty_module() {
        let m = compile("(module)");
        assert!(m.exports().is_empty());
        assert!(m.imports().is_empty());
    }

    #[test]
    fn custom_sections_returns_payload() {
        // (module (@custom "elidex-meta" "payload")) — wat-spec custom
        // section syntax.  We use a hand-crafted wasm binary because
        // `wat` may not parse `@custom` directives uniformly across
        // versions.
        //
        // Layout: header (8) + custom section (id=0):
        //   id   payload_size  name_len  name(11)            data(7)
        //   00   13            0b        "elidex-meta"        "payload"
        let mut bytes: Vec<u8> = Vec::new();
        bytes.extend_from_slice(b"\0asm");
        bytes.extend_from_slice(&[0x01, 0x00, 0x00, 0x00]);
        let name = b"elidex-meta";
        let data = b"payload";
        let payload_size = 1 + name.len() + data.len(); // name_len LEB + name + data
        bytes.push(0); // custom section id
        bytes.push(u8::try_from(payload_size).unwrap());
        bytes.push(u8::try_from(name.len()).unwrap());
        bytes.extend_from_slice(name);
        bytes.extend_from_slice(data);

        // Verify the binary at least parses as a module
        let inner = wasmtime::Module::new(&engine(), &bytes).unwrap();
        let m = WasmModule {
            inner,
            source_bytes: Arc::from(bytes.into_boxed_slice()),
        };

        let sections = m.custom_sections("elidex-meta");
        assert_eq!(sections, vec![data.to_vec()]);
        assert!(m.custom_sections("missing").is_empty());
    }

    #[test]
    fn custom_sections_multiple_same_name_preserves_order() {
        let mut bytes: Vec<u8> = Vec::new();
        bytes.extend_from_slice(b"\0asm");
        bytes.extend_from_slice(&[0x01, 0x00, 0x00, 0x00]);
        for data in &[b"first".as_ref(), b"second".as_ref()] {
            let name = b"dup";
            let payload_size = 1 + name.len() + data.len();
            bytes.push(0);
            bytes.push(u8::try_from(payload_size).unwrap());
            bytes.push(u8::try_from(name.len()).unwrap());
            bytes.extend_from_slice(name);
            bytes.extend_from_slice(data);
        }
        let inner = wasmtime::Module::new(&engine(), &bytes).unwrap();
        let m = WasmModule {
            inner,
            source_bytes: Arc::from(bytes.into_boxed_slice()),
        };

        let sections = m.custom_sections("dup");
        assert_eq!(sections.len(), 2);
        assert_eq!(sections[0], b"first");
        assert_eq!(sections[1], b"second");
    }

    #[test]
    fn read_leb128_single_byte() {
        assert_eq!(read_leb128_u32(&[0x05]), Some((5, 1)));
    }

    #[test]
    fn read_leb128_two_bytes() {
        // 200 = 0b11001000 → LEB128: 0xc8, 0x01
        assert_eq!(read_leb128_u32(&[0xc8, 0x01]), Some((200, 2)));
    }

    #[test]
    fn read_leb128_truncated_returns_none() {
        assert_eq!(read_leb128_u32(&[0x80]), None);
    }
}
