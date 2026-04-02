//! Bytecode disassembler for debugging and testing.

use std::fmt::{self, Write as _};

use super::compiled::{CompiledFunction, CompiledScript, Constant};
use super::opcode::Op;

/// Disassemble a compiled script into a human-readable string.
#[must_use]
pub fn disassemble_script(script: &CompiledScript) -> String {
    let mut out = String::new();
    out.push_str("=== Script ===\n");
    disassemble_function(&script.top_level, "<script>", &mut out, 0);
    out
}

/// Disassemble a single function.
#[allow(clippy::cast_possible_truncation, clippy::cast_possible_wrap)]
fn disassemble_function(func: &CompiledFunction, name: &str, out: &mut String, indent: usize) {
    let prefix = " ".repeat(indent);
    let _ = writeln!(
        out,
        "{prefix}--- {name} (locals={}, params={}, upvalues={}{}{}{}) ---",
        func.local_count,
        func.param_count,
        func.upvalues.len(),
        if func.is_async { ", async" } else { "" },
        if func.is_generator { ", generator" } else { "" },
        if func.is_strict { ", strict" } else { "" },
    );

    let mut pc = 0usize;
    while pc < func.bytecode.len() {
        let byte = func.bytecode[pc];
        let Some(op) = Op::from_byte(byte) else {
            let _ = writeln!(out, "{prefix}  {pc:04}: <invalid 0x{byte:02x}>");
            pc += 1;
            continue;
        };

        let operand_size = op.operand_size();
        let mut operand_str = String::new();

        match operand_size {
            0 | 5.. => {}
            1 => {
                if pc + 1 < func.bytecode.len() {
                    let val = func.bytecode[pc + 1];
                    if matches!(op, Op::PushI8) {
                        operand_str = format!(" {}", val as i8);
                    } else {
                        operand_str = format!(" {val}");
                    }
                }
            }
            2 => {
                if pc + 2 < func.bytecode.len() {
                    let val = read_u16(&func.bytecode, pc + 1);
                    if matches!(
                        op,
                        Op::Jump
                            | Op::JumpIfFalse
                            | Op::JumpIfTrue
                            | Op::JumpIfNullish
                            | Op::JumpIfNotNullish
                            | Op::DefaultIfUndefined
                    ) {
                        let offset = val as i16;
                        let target = (pc as i32) + 3 + i32::from(offset);
                        operand_str = format!(" {offset} (-> {target})");
                    } else {
                        operand_str = format!(" {val}");
                        // Annotate with constant value if it's a constant reference.
                        if matches!(
                            op,
                            Op::PushConst
                                | Op::GetGlobal
                                | Op::SetGlobal
                                | Op::GetProp
                                | Op::SetProp
                                | Op::TypeOfGlobal
                        ) {
                            if let Some(constant) = func.constants.get(val as usize) {
                                let _ = write!(operand_str, " ; {}", format_constant(constant));
                            }
                        }
                    }
                }
            }
            3 => {
                if pc + 4 <= func.bytecode.len() {
                    let val = read_u16(&func.bytecode, pc + 1);
                    let flag = func.bytecode[pc + 3];
                    operand_str = format!(" {val}, {flag}");
                }
            }
            4 => {
                if pc + 5 <= func.bytecode.len() {
                    let a = read_u16(&func.bytecode, pc + 1);
                    let b = read_u16(&func.bytecode, pc + 3);
                    operand_str = format!(" {a}, {b}");
                }
            }
        }

        let _ = writeln!(out, "{prefix}  {pc:04}: {op:?}{operand_str}");
        pc += 1 + operand_size;
    }

    // Disassemble nested functions.
    for (i, constant) in func.constants.iter().enumerate() {
        if let Constant::Function(nested) = constant {
            let fallback = format!("<fn#{i}>");
            let nested_name = nested.name.as_deref().unwrap_or(&fallback);
            out.push('\n');
            disassemble_function(nested, nested_name, out, indent + 2);
        }
    }
}

/// Format a constant for display.
fn format_constant(c: &Constant) -> String {
    match c {
        Constant::Number(n) => format!("{n}"),
        Constant::String(s) => format!("\"{s}\""),
        Constant::BigInt(s) => format!("{s}n"),
        Constant::Function(_) => "<function>".to_string(),
        Constant::RegExp { pattern, flags } => format!("/{pattern}/{flags}"),
        Constant::TemplateObject { .. } => "<template>".to_string(),
    }
}

/// Read a little-endian u16 from bytecode at the given offset.
fn read_u16(bytecode: &[u8], offset: usize) -> u16 {
    u16::from_le_bytes([bytecode[offset], bytecode[offset + 1]])
}

/// Display implementation for `CompiledScript`.
impl fmt::Display for CompiledScript {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&disassemble_script(self))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bytecode::compiled::CompiledFunction;
    use crate::bytecode::source_map::SourceMap;

    fn simple_function(bytecode: Vec<u8>, constants: Vec<Constant>) -> CompiledFunction {
        CompiledFunction {
            bytecode,
            constants,
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

    #[test]
    fn disasm_empty() {
        let script = CompiledScript {
            top_level: simple_function(vec![], vec![]),
            source: String::new(),
            line_starts: vec![0],
        };
        let output = disassemble_script(&script);
        assert!(output.contains("<script>"));
        assert!(output.contains("locals=0"));
    }

    #[test]
    fn disasm_push_const() {
        let script = CompiledScript {
            top_level: simple_function(
                vec![Op::PushConst as u8, 0, 0, Op::Return as u8],
                vec![Constant::Number(42.0)],
            ),
            source: String::new(),
            line_starts: vec![0],
        };
        let output = disassemble_script(&script);
        assert!(output.contains("PushConst"));
        assert!(output.contains("42"));
        assert!(output.contains("Return"));
    }

    #[test]
    fn disasm_jump() {
        let script = CompiledScript {
            top_level: simple_function(vec![Op::Jump as u8, 3, 0, Op::Pop as u8], vec![]),
            source: String::new(),
            line_starts: vec![0],
        };
        let output = disassemble_script(&script);
        assert!(output.contains("Jump"));
        assert!(output.contains("-> 6"));
    }
}
