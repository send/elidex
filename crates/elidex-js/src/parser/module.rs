//! Module declaration parser: import/export.

use crate::arena::NodeId;
#[allow(clippy::wildcard_imports)]
use crate::ast::*;
use crate::atom::Atom;
use crate::error::JsParseErrorKind;
use crate::span::Span;
use crate::token::{Keyword, TokenKind};

use super::Parser;

impl Parser<'_> {
    // ── Import Declaration ───────────────────────────────────────────

    pub(crate) fn parse_import_declaration(&mut self) -> NodeId<Stmt> {
        let start = self.span();
        self.advance(); // skip `import`

        // `import 'module'` — side-effect only
        if let TokenKind::StringLiteral(source) = *self.at() {
            self.advance();
            self.expect_semicolon();
            let end = self.span();
            return self.stmts.alloc(Stmt {
                kind: StmtKind::ImportDeclaration(ImportDecl {
                    specifiers: Vec::new(),
                    source,
                    span: start.merge(end),
                }),
                span: start.merge(end),
            });
        }

        let mut specifiers = Vec::new();

        // `import * as ns from 'mod'`
        if matches!(*self.at(), TokenKind::Star) {
            self.advance();
            let _ = self.expect_contextual_atom(self.atoms.r#as);
            if let TokenKind::Identifier(local) = *self.at() {
                let ns_span = self.span();
                self.advance();
                specifiers.push(ImportSpecifier::Namespace(local, ns_span));
            } else {
                self.error(
                    JsParseErrorKind::UnexpectedToken,
                    "Expected identifier after 'as'".into(),
                );
            }
        }
        // `import { a, b as c } from 'mod'`
        else if matches!(*self.at(), TokenKind::LBrace) {
            self.parse_named_imports(&mut specifiers);
        }
        // `import x from 'mod'` or `import x, { a } from 'mod'`
        else if let TokenKind::Identifier(name) = *self.at() {
            let def_span = self.span();
            self.advance();
            specifiers.push(ImportSpecifier::Default(name, def_span));

            if matches!(*self.at(), TokenKind::Comma) {
                self.advance();
                if matches!(*self.at(), TokenKind::Star) {
                    // `import x, * as ns from 'mod'`
                    self.advance();
                    let _ = self.expect_contextual_atom(self.atoms.r#as);
                    if let TokenKind::Identifier(local) = *self.at() {
                        let ns_span = self.span();
                        self.advance();
                        specifiers.push(ImportSpecifier::Namespace(local, ns_span));
                    }
                } else if matches!(*self.at(), TokenKind::LBrace) {
                    self.parse_named_imports(&mut specifiers);
                }
            }
        } else {
            self.error(
                JsParseErrorKind::UnexpectedToken,
                "Invalid import declaration".into(),
            );
        }

        let _ = self.expect_contextual_atom(self.atoms.from);
        let source = if let TokenKind::StringLiteral(s) = *self.at() {
            self.advance();
            s
        } else {
            self.error(
                JsParseErrorKind::UnexpectedToken,
                "Expected module specifier string".into(),
            );
            Atom::EMPTY
        };
        self.expect_semicolon();

        let end = self.span();
        self.stmts.alloc(Stmt {
            kind: StmtKind::ImportDeclaration(ImportDecl {
                specifiers,
                source,
                span: start.merge(end),
            }),
            span: start.merge(end),
        })
    }

    fn parse_named_imports(&mut self, specifiers: &mut Vec<ImportSpecifier>) {
        let _ = self.expect(&TokenKind::LBrace);
        while !matches!(*self.at(), TokenKind::RBrace | TokenKind::Eof) && !self.aborted {
            let spec_start = self.span();
            // P4: track if imported name is a string literal (requires `as` for local binding)
            let is_string_import = matches!(*self.at(), TokenKind::StringLiteral(_));
            let imported = self.parse_module_export_name();

            let local = if self.at_contextual_atom(self.atoms.r#as) {
                self.advance();
                if let TokenKind::Identifier(name) = *self.at() {
                    self.advance();
                    name
                } else {
                    self.error(
                        JsParseErrorKind::UnexpectedToken,
                        "Expected identifier after 'as'".into(),
                    );
                    imported
                }
            } else {
                // P4: string import must use 'as' to bind to a local name
                if is_string_import {
                    self.error(
                        JsParseErrorKind::UnexpectedToken,
                        "String literal import must use 'as' to bind to a local name".into(),
                    );
                }
                imported
            };

            let end = self.span();
            specifiers.push(ImportSpecifier::Named {
                imported,
                local,
                span: spec_start.merge(end),
            });
            if self.check_list_limit(specifiers.len(), "import specifiers") {
                break;
            }

            self.expect_comma_unless(&TokenKind::RBrace);
        }
        let _ = self.expect(&TokenKind::RBrace);
    }

    /// Parse a module export name (identifier or string literal).
    fn parse_module_export_name(&mut self) -> Atom {
        match *self.at() {
            TokenKind::Identifier(name) => {
                self.advance();
                name
            }
            TokenKind::Keyword(kw) => {
                // L9: use canonical keyword string, not Debug format
                let name = self.intern(kw.as_str());
                self.advance();
                name
            }
            TokenKind::StringLiteral(s) => {
                self.advance();
                s
            }
            _ => {
                self.error(
                    JsParseErrorKind::UnexpectedToken,
                    format!("Expected export name, got {:?}", self.at()),
                );
                self.advance();
                Atom::EMPTY
            }
        }
    }

    // ── Export Declaration ────────────────────────────────────────────

    pub(crate) fn parse_export_declaration(&mut self) -> NodeId<Stmt> {
        let start = self.span();
        self.advance(); // skip `export`

        // `export default ...`
        if self.at_keyword(Keyword::Default) {
            return self.parse_export_default(start);
        }

        // `export * from 'mod'` or `export * as ns from 'mod'`
        if matches!(*self.at(), TokenKind::Star) {
            self.advance();
            if self.at_contextual_atom(self.atoms.r#as) {
                self.advance();
                let exported = self.parse_module_export_name();
                let _ = self.expect_contextual_atom(self.atoms.from);
                let source = self.expect_string_literal();
                self.expect_semicolon();
                let end = self.span();
                return self.alloc_export(
                    ExportKind::NamespaceFrom { exported, source },
                    start,
                    end,
                );
            }
            let _ = self.expect_contextual_atom(self.atoms.from);
            let source = self.expect_string_literal();
            self.expect_semicolon();
            let end = self.span();
            return self.alloc_export(ExportKind::AllFrom { source }, start, end);
        }

        // `export { a, b as c } [from 'mod']`
        if matches!(*self.at(), TokenKind::LBrace) {
            let (specifiers, has_string_local) = self.parse_export_specifiers();
            let source = if self.at_contextual_atom(self.atoms.from) {
                self.advance();
                Some(self.expect_string_literal())
            } else {
                // P5: string literal as local name requires `from` (re-export)
                if has_string_local {
                    self.error(
                        JsParseErrorKind::UnexpectedToken,
                        "String literal export name requires 'from' clause".into(),
                    );
                }
                None
            };
            self.expect_semicolon();
            let end = self.span();
            return self.alloc_export(ExportKind::Named { specifiers, source }, start, end);
        }

        // `export var/let/const ...`
        if matches!(
            *self.at(),
            TokenKind::Keyword(Keyword::Var | Keyword::Let | Keyword::Const)
        ) {
            let decl = self.parse_variable_declaration_stmt();
            return self.wrap_export_decl(decl, start);
        }

        // `export function ...`
        if matches!(*self.at(), TokenKind::Keyword(Keyword::Function)) {
            let decl = self.parse_function_declaration();
            return self.wrap_export_decl(decl, start);
        }

        // `export class ...`
        if matches!(*self.at(), TokenKind::Keyword(Keyword::Class)) {
            let decl = self.parse_class_declaration();
            return self.wrap_export_decl(decl, start);
        }

        // `export async function ...`
        // A17: check [no LineTerminator here] between async and function
        if self.at_contextual_atom(self.atoms.r#async)
            && matches!(self.peek_kind(), TokenKind::Keyword(Keyword::Function))
            && self.peek_no_newline()
        {
            let decl = self.parse_async_function_declaration();
            return self.wrap_export_decl(decl, start);
        }

        self.error(
            JsParseErrorKind::UnexpectedToken,
            format!("Invalid export declaration, got {:?}", self.at()),
        );
        self.error_stmt(start)
    }

    fn parse_export_default(&mut self, start: Span) -> NodeId<Stmt> {
        self.advance(); // skip `default`

        // `export default function [name] () {}`
        if matches!(*self.at(), TokenKind::Keyword(Keyword::Function)) {
            let func = self.parse_function_inner(false);
            let end = func.span;
            return self.alloc_export(ExportKind::DefaultFunction(func), start, end);
        }

        // `export default class [name] {}`
        if matches!(*self.at(), TokenKind::Keyword(Keyword::Class)) {
            let class = self.parse_class_inner();
            let end = class.span;
            return self.alloc_export(ExportKind::DefaultClass(class), start, end);
        }

        // `export default async function [name] () {}`
        if self.at_contextual_atom(self.atoms.r#async) {
            let async_start = self.span();
            self.advance(); // skip async
            if matches!(*self.at(), TokenKind::Keyword(Keyword::Function))
                && !self.had_newline_before
            {
                let func = self.parse_function_inner(true);
                let end = func.span;
                return self.alloc_export(ExportKind::DefaultFunction(func), start, end);
            }
            // Not async function — `async` was the expression
            let async_atom = self.atoms.r#async;
            let async_expr = self.exprs.alloc(Expr {
                kind: ExprKind::Identifier(async_atom),
                span: async_start,
            });
            let expr = self.continue_expression_from(async_expr);
            self.expect_semicolon();
            let end = self.exprs.get(expr).span;
            return self.alloc_export(ExportKind::Default(expr), start, end);
        }

        // `export default expr`
        let expr = self.parse_assignment_expression();
        self.expect_semicolon();
        let end = self.exprs.get(expr).span;
        self.alloc_export(ExportKind::Default(expr), start, end)
    }

    fn parse_export_specifiers(&mut self) -> (Vec<ExportSpecifier>, bool) {
        let _ = self.expect(&TokenKind::LBrace);
        let mut specifiers = Vec::new();
        let mut has_string_local = false;
        while !matches!(*self.at(), TokenKind::RBrace | TokenKind::Eof) && !self.aborted {
            let spec_start = self.span();
            // P5: track if local name is a string literal
            let is_string_local = matches!(*self.at(), TokenKind::StringLiteral(_));
            let local = self.parse_module_export_name();
            let exported = if self.at_contextual_atom(self.atoms.r#as) {
                self.advance();
                self.parse_module_export_name()
            } else {
                if is_string_local {
                    has_string_local = true;
                }
                local
            };
            let end = self.span();
            specifiers.push(ExportSpecifier {
                local,
                exported,
                span: spec_start.merge(end),
            });
            if self.check_list_limit(specifiers.len(), "export specifiers") {
                break;
            }
            self.expect_comma_unless(&TokenKind::RBrace);
        }
        let _ = self.expect(&TokenKind::RBrace);
        (specifiers, has_string_local)
    }

    fn expect_string_literal(&mut self) -> Atom {
        if let TokenKind::StringLiteral(s) = *self.at() {
            self.advance();
            s
        } else {
            self.error(
                JsParseErrorKind::UnexpectedToken,
                "Expected string literal".into(),
            );
            // S5: advance past the unexpected token to prevent cascading errors
            self.advance();
            Atom::EMPTY
        }
    }

    /// Wrap a declaration statement as `export <decl>`.
    fn wrap_export_decl(&mut self, decl: NodeId<Stmt>, start: Span) -> NodeId<Stmt> {
        let end = self.stmts.get(decl).span;
        self.alloc_export(ExportKind::Declaration(decl), start, end)
    }

    /// R5: Allocate an export declaration statement.
    fn alloc_export(&mut self, kind: ExportKind, start: Span, end: Span) -> NodeId<Stmt> {
        self.stmts.alloc(Stmt {
            kind: StmtKind::ExportDeclaration(ExportDecl {
                kind,
                span: start.merge(end),
            }),
            span: start.merge(end),
        })
    }
}

#[cfg(test)]
mod tests {
    use crate::{ast::*, parse_module};

    fn r(program: &Program, atom: crate::atom::Atom) -> &str {
        program.interner.get(atom)
    }

    #[test]
    fn import_default() {
        let out = parse_module("import x from 'mod';");
        assert!(out.errors.is_empty(), "{:?}", out.errors);
        if let StmtKind::ImportDeclaration(decl) = &out.program.stmts.get(out.program.body[0]).kind
        {
            assert_eq!(r(&out.program, decl.source), "mod");
            assert!(
                matches!(&decl.specifiers[0], ImportSpecifier::Default(name, _) if r(&out.program, *name) == "x")
            );
        } else {
            panic!("Expected import");
        }
    }

    #[test]
    fn import_namespace() {
        let out = parse_module("import * as ns from 'mod';");
        assert!(out.errors.is_empty(), "{:?}", out.errors);
    }

    #[test]
    fn import_named() {
        let out = parse_module("import { a, b as c } from 'mod';");
        assert!(out.errors.is_empty(), "{:?}", out.errors);
        if let StmtKind::ImportDeclaration(decl) = &out.program.stmts.get(out.program.body[0]).kind
        {
            assert_eq!(decl.specifiers.len(), 2);
        }
    }

    #[test]
    fn import_side_effect() {
        let out = parse_module("import 'mod';");
        assert!(out.errors.is_empty(), "{:?}", out.errors);
        if let StmtKind::ImportDeclaration(decl) = &out.program.stmts.get(out.program.body[0]).kind
        {
            assert!(decl.specifiers.is_empty());
        }
    }

    #[test]
    fn import_default_and_named() {
        let out = parse_module("import x, { a } from 'mod';");
        assert!(out.errors.is_empty(), "{:?}", out.errors);
        if let StmtKind::ImportDeclaration(decl) = &out.program.stmts.get(out.program.body[0]).kind
        {
            assert_eq!(decl.specifiers.len(), 2);
        }
    }

    #[test]
    fn export_named() {
        let out = parse_module("export { a, b as c };");
        assert!(out.errors.is_empty(), "{:?}", out.errors);
    }

    #[test]
    fn export_default_expression() {
        let out = parse_module("export default 42;");
        assert!(out.errors.is_empty(), "{:?}", out.errors);
    }

    #[test]
    fn export_default_function() {
        let out = parse_module("export default function foo() {}");
        assert!(out.errors.is_empty(), "{:?}", out.errors);
    }

    #[test]
    fn export_star_from() {
        let out = parse_module("export * from 'mod';");
        assert!(out.errors.is_empty(), "{:?}", out.errors);
    }

    #[test]
    fn export_namespace_from() {
        let out = parse_module("export * as ns from 'mod';");
        assert!(out.errors.is_empty(), "{:?}", out.errors);
    }

    #[test]
    fn export_declaration() {
        let out = parse_module("export const x = 1;");
        assert!(out.errors.is_empty(), "{:?}", out.errors);
    }

    #[test]
    fn export_re_export() {
        let out = parse_module("export { a, b } from 'mod';");
        assert!(out.errors.is_empty(), "{:?}", out.errors);
    }

    // ── Step 3: L9 — export keyword as module name ──

    #[test]
    fn export_keyword_as_name() {
        // `export { default as default } from 'mod';` — keyword as module export name
        let out = parse_module("export { default as default } from 'mod';");
        assert!(out.errors.is_empty(), "{:?}", out.errors);
    }
}
