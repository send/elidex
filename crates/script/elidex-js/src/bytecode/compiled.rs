//! Compilation unit structures for the elidex-js bytecode.

use super::source_map::SourceMap;

/// A compiled script or module — the top-level compilation unit.
#[derive(Debug)]
pub struct CompiledScript {
    /// The top-level function (script body or module body).
    pub top_level: CompiledFunction,
    /// Original source text (kept for error messages).
    pub source: String,
    /// Byte offsets of line starts in `source`, for line:column computation.
    pub line_starts: Vec<u32>,
}

impl CompiledScript {
    /// Compute line and column (both 1-based) from a byte offset in source.
    #[must_use]
    pub fn location(&self, offset: u32) -> (u32, u32) {
        let line = self.line_starts.partition_point(|&start| start <= offset);
        let line_start = if line > 0 {
            self.line_starts[line - 1]
        } else {
            0
        };
        let col = offset - line_start + 1;
        (line as u32, col)
    }

    /// Build line_starts from source text.
    #[must_use]
    pub fn compute_line_starts(source: &str) -> Vec<u32> {
        let mut starts = vec![0];
        for (i, b) in source.bytes().enumerate() {
            if b == b'\n' {
                #[allow(clippy::cast_possible_truncation)]
                starts.push((i + 1) as u32);
            }
        }
        starts
    }
}

/// A single compiled function (or script body, class initializer, eval).
#[derive(Debug, Clone)]
#[allow(clippy::struct_excessive_bools)]
pub struct CompiledFunction {
    /// The bytecode instruction stream.
    pub bytecode: Vec<u8>,
    /// Constant pool for this function.
    pub constants: Vec<Constant>,
    /// Number of local variable slots (params + locals).
    pub local_count: u16,
    /// Number of parameters (not counting rest).
    pub param_count: u16,
    /// Upvalue descriptors: how to capture each upvalue.
    pub upvalues: Vec<UpvalueDesc>,
    /// Source map: bytecode offset → source Span.
    pub source_map: SourceMap,
    /// Function name (for stack traces).
    pub name: Option<String>,
    /// Exception handler table (sorted by start offset).
    pub exception_handlers: Vec<ExceptionHandler>,
    /// Flags.
    pub is_async: bool,
    pub is_generator: bool,
    pub is_arrow: bool,
    pub is_strict: bool,
}

impl CompiledFunction {
    /// Create a new empty compiled function.
    #[must_use]
    pub fn new() -> Self {
        Self {
            bytecode: Vec::new(),
            constants: Vec::new(),
            local_count: 0,
            param_count: 0,
            upvalues: Vec::new(),
            source_map: SourceMap::new(),
            name: None,
            exception_handlers: Vec::new(),
            is_async: false,
            is_generator: false,
            is_arrow: false,
            is_strict: false,
        }
    }
}

impl Default for CompiledFunction {
    fn default() -> Self {
        Self::new()
    }
}

/// Constant pool entry.
#[derive(Debug, Clone)]
pub enum Constant {
    /// f64 number literal.
    Number(f64),
    /// String value (identifier names, string literals).
    String(String),
    /// BigInt literal (string representation, parsed lazily by VM).
    BigInt(String),
    /// Nested compiled function (for closures, class methods).
    Function(Box<CompiledFunction>),
    /// RegExp pattern + flags.
    RegExp { pattern: String, flags: String },
    /// Template object (cooked + raw arrays for tagged templates).
    TemplateObject {
        cooked: Vec<Option<String>>,
        raw: Vec<String>,
    },
}

/// Describes how an upvalue is captured.
#[derive(Debug, Clone, Copy)]
pub struct UpvalueDesc {
    /// If true, captures from the immediately enclosing function's locals.
    /// If false, captures from the enclosing function's upvalues (transitive).
    pub is_local: bool,
    /// Index into either the parent's locals or parent's upvalues.
    pub index: u16,
}

/// Exception handler entry (try/catch/finally).
#[derive(Debug, Clone)]
pub struct ExceptionHandler {
    /// Bytecode range this handler covers `[start, end)`.
    pub start: u32,
    pub end: u32,
    /// Bytecode offset of catch block (`u32::MAX` if no catch).
    pub catch_offset: u32,
    /// Bytecode offset of finally block (`u32::MAX` if no finally).
    pub finally_offset: u32,
    /// Local slot for the catch parameter (if catch has a binding).
    pub catch_binding: Option<u16>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn line_starts() {
        let source = "hello\nworld\nfoo";
        let starts = CompiledScript::compute_line_starts(source);
        assert_eq!(starts, vec![0, 6, 12]);
    }

    #[test]
    fn location_first_line() {
        let script = CompiledScript {
            top_level: CompiledFunction::new(),
            source: "hello\nworld".into(),
            line_starts: vec![0, 6],
        };
        assert_eq!(script.location(0), (1, 1));
        assert_eq!(script.location(4), (1, 5));
    }

    #[test]
    fn location_second_line() {
        let script = CompiledScript {
            top_level: CompiledFunction::new(),
            source: "hello\nworld".into(),
            line_starts: vec![0, 6],
        };
        assert_eq!(script.location(6), (2, 1));
        assert_eq!(script.location(9), (2, 4));
    }

    #[test]
    fn compiled_function_default() {
        let f = CompiledFunction::new();
        assert!(f.bytecode.is_empty());
        assert!(f.constants.is_empty());
        assert_eq!(f.local_count, 0);
        assert!(!f.is_async);
    }

    #[test]
    fn upvalue_desc() {
        let uv = UpvalueDesc {
            is_local: true,
            index: 42,
        };
        assert!(uv.is_local);
        assert_eq!(uv.index, 42);
    }
}
