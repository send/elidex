//! AST-to-bytecode compiler for elidex-js (Stage 2).
//!
//! Compiles a parsed and scope-analyzed ES2020+ program into bytecode
//! that can be executed by the elidex-js interpreter (M4-10).

mod expr;
pub mod function;
pub mod resolve;
mod stmt;

use crate::ast::Program;
use crate::bytecode::compiled::CompiledScript;
use crate::bytecode::opcode::Op;
use crate::scope::ScopeAnalysis;

use function::FunctionCompiler;
use resolve::build_function_scopes;
use stmt::compile_stmt;

/// Compilation error.
#[derive(Debug)]
pub struct CompileError {
    pub message: String,
}

impl std::fmt::Display for CompileError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.message)
    }
}

impl std::error::Error for CompileError {}

/// Compile a parsed program into bytecode.
///
/// # Arguments
/// * `program` — The parsed AST (from [`parse_script`](crate::parse_script) or [`parse_module`](crate::parse_module))
/// * `analysis` — Scope analysis result (from [`analyze_scopes`](crate::analyze_scopes))
/// * `source` — Original source text (kept for error messages)
///
/// # Returns
/// A `CompiledScript` containing the top-level bytecode and all nested functions.
pub fn compile(
    program: &Program,
    analysis: &ScopeAnalysis,
    source: &str,
) -> Result<CompiledScript, CompileError> {
    let mut func_scopes = build_function_scopes(analysis);

    let is_strict = analysis.scopes.first().map_or(false, |s| s.is_strict);

    let mut fc = FunctionCompiler::new(0, is_strict);
    fc.name = Some("<script>".to_string());

    // Compile top-level statements.
    for &stmt_id in &program.body {
        compile_stmt(&mut fc, program, analysis, &mut func_scopes, stmt_id);
    }

    // Ensure the function ends with a return.
    if fc.bytecode.last() != Some(&(Op::Return as u8))
        && fc.bytecode.last() != Some(&(Op::ReturnUndefined as u8))
    {
        fc.emit(Op::ReturnUndefined);
    }

    let top_level = fc.finish(&func_scopes[0]);

    Ok(CompiledScript {
        top_level,
        source: source.to_string(),
        line_starts: CompiledScript::compute_line_starts(source),
    })
}

/// Convenience: parse + analyze + compile in one step.
pub fn compile_script(source: &str) -> Result<CompiledScript, CompileError> {
    let output = crate::parse_script(source);
    if !output.errors.is_empty() {
        return Err(CompileError {
            message: format!("parse errors: {:?}", output.errors),
        });
    }
    let analysis = crate::analyze_scopes(&output.program);
    if !analysis.errors.is_empty() {
        return Err(CompileError {
            message: format!("scope errors: {:?}", analysis.errors),
        });
    }
    compile(&output.program, &analysis, source)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bytecode::disasm::disassemble_script;

    fn compile_and_disasm(source: &str) -> String {
        let script = compile_script(source).expect("compilation failed");
        disassemble_script(&script)
    }

    #[test]
    fn compile_empty() {
        let output = compile_and_disasm("");
        assert!(output.contains("ReturnUndefined"));
    }

    #[test]
    fn compile_number_literal() {
        let output = compile_and_disasm("42;");
        assert!(output.contains("PushI8"));
        assert!(output.contains("Pop")); // expression statement discards
    }

    #[test]
    fn compile_large_number() {
        let output = compile_and_disasm("3.14;");
        assert!(output.contains("PushConst"));
    }

    #[test]
    fn compile_boolean() {
        let output = compile_and_disasm("true; false;");
        assert!(output.contains("PushTrue"));
        assert!(output.contains("PushFalse"));
    }

    #[test]
    fn compile_null_undefined() {
        let output = compile_and_disasm("null; undefined;");
        assert!(output.contains("PushNull"));
        assert!(output.contains("GetGlobal"));
    }

    #[test]
    fn compile_binary() {
        let output = compile_and_disasm("1 + 2;");
        assert!(output.contains("Add"));
    }

    #[test]
    fn compile_unary() {
        let output = compile_and_disasm("-x;");
        assert!(output.contains("Neg"));
    }

    #[test]
    fn compile_var_decl() {
        let output = compile_and_disasm("var x = 10;");
        assert!(output.contains("SetLocal"));
    }

    #[test]
    fn compile_let_decl() {
        let output = compile_and_disasm("let x = 10;");
        assert!(output.contains("InitLocal"));
        assert!(output.contains("SetLocal"));
    }

    #[test]
    fn compile_if_else() {
        let output = compile_and_disasm("if (x) { y; } else { z; }");
        assert!(output.contains("JumpIfFalse"));
        assert!(output.contains("Jump"));
    }

    #[test]
    fn compile_while_loop() {
        let output = compile_and_disasm("while (true) { x; }");
        assert!(output.contains("JumpIfFalse"));
        // Should have a backward jump.
        assert!(output.contains("Jump"));
    }

    #[test]
    fn compile_for_loop() {
        let output = compile_and_disasm("for (var i = 0; i < 10; i++) { x; }");
        assert!(output.contains("SetLocal"));
        assert!(output.contains("Lt"));
        assert!(output.contains("JumpIfFalse"));
    }

    #[test]
    fn compile_return() {
        // `return` must be inside a function; top-level return is a parse error.
        // For now, test that the compiled bytecode ends with ReturnUndefined.
        let output = compile_and_disasm("42;");
        assert!(output.contains("ReturnUndefined"));
    }

    #[test]
    fn compile_function_call() {
        let output = compile_and_disasm("console.log(42);");
        assert!(output.contains("GetGlobal"));
        assert!(output.contains("GetProp"));
        assert!(output.contains("CallMethod"));
    }

    #[test]
    fn compile_logical_and() {
        let output = compile_and_disasm("a && b;");
        assert!(output.contains("Dup"));
        assert!(output.contains("JumpIfFalse"));
    }

    #[test]
    fn compile_conditional() {
        let output = compile_and_disasm("x ? y : z;");
        assert!(output.contains("JumpIfFalse"));
    }

    #[test]
    fn compile_try_catch() {
        let output = compile_and_disasm("try { x; } catch(e) { y; }");
        assert!(output.contains("PushExceptionHandler"));
        assert!(output.contains("PopExceptionHandler"));
        assert!(output.contains("PushException"));
    }

    #[test]
    fn compile_throw() {
        let output = compile_and_disasm("throw new Error();");
        assert!(output.contains("Throw"));
    }

    #[test]
    fn compile_array_literal() {
        let output = compile_and_disasm("[1, 2, 3];");
        assert!(output.contains("CreateArray"));
        assert!(output.contains("ArrayPush"));
    }

    #[test]
    fn compile_object_literal() {
        let output = compile_and_disasm("({a: 1, b: 2});");
        assert!(output.contains("CreateObject"));
        assert!(output.contains("DefineProperty"));
    }

    #[test]
    fn compile_assignment() {
        let output = compile_and_disasm("var x; x = 10;");
        assert!(output.contains("SetLocal"));
    }

    #[test]
    fn compile_compound_assignment() {
        let output = compile_and_disasm("var x = 1; x += 2;");
        assert!(output.contains("GetLocal"));
        assert!(output.contains("Add"));
        assert!(output.contains("SetLocal"));
    }

    #[test]
    fn compile_member_access() {
        let output = compile_and_disasm("obj.prop;");
        assert!(output.contains("GetProp"));
    }

    #[test]
    fn compile_typeof_global() {
        let output = compile_and_disasm("typeof x;");
        assert!(output.contains("TypeOfGlobal"));
    }
}
