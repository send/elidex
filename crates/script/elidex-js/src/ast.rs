//! AST node definitions for ES2020+ JavaScript.
//!
//! All nodes are Arena-allocated; cross-references use typed `NodeId<T>`.
//! Every node carries a `Span` for source mapping.
//! All string data uses `Atom` (interned handles) for zero-copy sharing.

use crate::arena::{Arena, NodeId};
use crate::atom::{Atom, StringInterner, WellKnownAtoms};
use crate::span::Span;

// ── Program ──────────────────────────────────────────────────────────

/// Top-level program node owning all arenas.
#[derive(Debug)]
pub struct Program {
    pub kind: ProgramKind,
    pub body: Vec<NodeId<Stmt>>,
    pub stmts: Arena<Stmt>,
    pub exprs: Arena<Expr>,
    pub patterns: Arena<Pattern>,
    /// Shared string interner — resolve `Atom` values via `interner.get(atom)`.
    pub interner: StringInterner,
    /// Pre-interned atoms for frequently used names.
    pub atoms: WellKnownAtoms,
}

/// Script or Module.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProgramKind {
    Script,
    Module,
}

// ── Statements ───────────────────────────────────────────────────────

/// A statement node.
#[derive(Debug, Clone)]
pub struct Stmt {
    pub kind: StmtKind,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub enum StmtKind {
    /// `var`/`let`/`const` declarations.
    VariableDeclaration {
        kind: VarKind,
        declarators: Vec<Declarator>,
    },
    /// Function declaration.
    FunctionDeclaration(Box<Function>),
    /// Class declaration.
    ClassDeclaration(Box<Class>),
    /// Block `{ ... }`.
    Block(Vec<NodeId<Stmt>>),
    /// Empty statement `;`.
    Empty,
    /// Expression statement.
    Expression(NodeId<Expr>),
    /// `if (test) consequent [else alternate]`.
    If {
        test: NodeId<Expr>,
        consequent: NodeId<Stmt>,
        alternate: Option<NodeId<Stmt>>,
    },
    /// `while (test) body`.
    While {
        test: NodeId<Expr>,
        body: NodeId<Stmt>,
    },
    /// `do body while (test)`.
    DoWhile {
        body: NodeId<Stmt>,
        test: NodeId<Expr>,
    },
    /// `for (init; test; update) body`.
    For {
        init: Option<ForInit>,
        test: Option<NodeId<Expr>>,
        update: Option<NodeId<Expr>>,
        body: NodeId<Stmt>,
    },
    /// `for (left in right) body`.
    ForIn {
        left: ForInOfLeft,
        right: NodeId<Expr>,
        body: NodeId<Stmt>,
    },
    /// `for (left of right) body` / `for await (left of right) body`.
    ForOf {
        is_await: bool,
        left: ForInOfLeft,
        right: NodeId<Expr>,
        body: NodeId<Stmt>,
    },
    /// `switch (discriminant) { cases }`.
    Switch {
        discriminant: NodeId<Expr>,
        cases: Vec<SwitchCase>,
    },
    /// `return [expr]`.
    Return(Option<NodeId<Expr>>),
    /// `throw expr`.
    Throw(NodeId<Expr>),
    /// `try { block } [catch (param) { block }] [finally { block }]`.
    Try {
        block: Vec<NodeId<Stmt>>,
        handler: Option<CatchClause>,
        finalizer: Option<Vec<NodeId<Stmt>>>,
    },
    /// `break [label]`.
    Break(Option<Atom>),
    /// `continue [label]`.
    Continue(Option<Atom>),
    /// `label: stmt`.
    Labeled { label: Atom, body: NodeId<Stmt> },
    /// `with (object) body` — always errors in strict mode (elidex is strict-only).
    With {
        object: NodeId<Expr>,
        body: NodeId<Stmt>,
    },
    /// `debugger`.
    Debugger,

    // ── Module declarations ──────────────────────────────────────
    /// `import` declaration.
    ImportDeclaration(ImportDecl),
    /// `export` declaration.
    ExportDeclaration(ExportDecl),

    /// Error recovery placeholder.
    Error,
}

// ── Statement sub-types ──────────────────────────────────────────────

/// `var` / `let` / `const`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VarKind {
    Var,
    Let,
    Const,
}

/// Single declarator: `pattern = init`.
#[derive(Debug, Clone)]
pub struct Declarator {
    pub pattern: NodeId<Pattern>,
    pub init: Option<NodeId<Expr>>,
    pub span: Span,
}

/// For-loop init can be either a declaration or an expression.
#[derive(Debug, Clone)]
pub enum ForInit {
    Declaration {
        kind: VarKind,
        declarators: Vec<Declarator>,
    },
    Expression(NodeId<Expr>),
}

/// Left-hand side of `for-in` / `for-of`.
#[derive(Debug, Clone)]
pub enum ForInOfLeft {
    Declaration {
        kind: VarKind,
        pattern: NodeId<Pattern>,
    },
    Pattern(NodeId<Expr>),
}

/// A single `case`/`default` clause.
#[derive(Debug, Clone)]
pub struct SwitchCase {
    /// `None` for `default`.
    pub test: Option<NodeId<Expr>>,
    pub consequent: Vec<NodeId<Stmt>>,
    pub span: Span,
}

/// Catch clause.
#[derive(Debug, Clone)]
pub struct CatchClause {
    /// `None` for optional catch binding (ES2019).
    pub param: Option<NodeId<Pattern>>,
    pub body: Vec<NodeId<Stmt>>,
    pub span: Span,
}

// ── Expressions ──────────────────────────────────────────────────────

/// An expression node.
#[derive(Debug, Clone)]
pub struct Expr {
    pub kind: ExprKind,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub enum ExprKind {
    /// Identifier reference.
    Identifier(Atom),
    /// Literal value.
    Literal(Literal),
    /// `this`.
    This,
    /// `super` (member access / call in derived constructors).
    Super,
    /// Array literal `[a, b, ...c]`.
    Array(Vec<Option<ArrayElement>>),
    /// Object literal `{ key: value, ...rest }`.
    Object(Vec<Property>),
    /// Function expression.
    Function(Box<Function>),
    /// Class expression.
    Class(Box<Class>),
    /// Template literal `` `a ${b} c` ``.
    Template(TemplateLiteral),
    /// Tagged template `` tag`a ${b}` ``.
    TaggedTemplate {
        tag: NodeId<Expr>,
        template: TemplateLiteral,
    },
    /// Unary: `!x`, `typeof x`, `-x`, etc.
    Unary { op: UnaryOp, argument: NodeId<Expr> },
    /// Update: `x++`, `--x`.
    Update {
        op: UpdateOp,
        prefix: bool,
        argument: NodeId<Expr>,
    },
    /// Binary: `a + b`, `a instanceof b`, etc.
    Binary {
        left: NodeId<Expr>,
        op: BinaryOp,
        right: NodeId<Expr>,
    },
    /// Logical: `a && b`, `a || b`, `a ?? b`.
    Logical {
        left: NodeId<Expr>,
        op: LogicalOp,
        right: NodeId<Expr>,
    },
    /// Assignment: `a = b`, `a += b`.
    Assignment {
        left: AssignTarget,
        op: AssignOp,
        right: NodeId<Expr>,
    },
    /// Conditional: `test ? consequent : alternate`.
    Conditional {
        test: NodeId<Expr>,
        consequent: NodeId<Expr>,
        alternate: NodeId<Expr>,
    },
    /// Member access: `obj.prop` or `obj[expr]`.
    Member {
        object: NodeId<Expr>,
        property: MemberProp,
        computed: bool,
    },
    /// Optional chain: `obj?.prop`, `obj?.[expr]`, `obj?.()`.
    OptionalChain {
        base: NodeId<Expr>,
        chain: Vec<OptionalChainPart>,
    },
    /// Function call: `callee(args)`.
    Call {
        callee: NodeId<Expr>,
        arguments: Vec<Argument>,
    },
    /// `new Ctor(args)`.
    New {
        callee: NodeId<Expr>,
        arguments: Vec<Argument>,
    },
    /// `import(source)` or `import(source, options)` (ES2020/ES2025 dynamic import).
    DynamicImport {
        source: NodeId<Expr>,
        options: Option<NodeId<Expr>>,
    },
    /// Arrow function.
    Arrow(Box<ArrowFunction>),
    /// Spread element: `...expr`.
    Spread(NodeId<Expr>),
    /// `yield [expr]` / `yield* expr`.
    Yield {
        argument: Option<NodeId<Expr>>,
        delegate: bool,
    },
    /// `await expr`.
    Await(NodeId<Expr>),
    /// Comma-separated sequence.
    Sequence(Vec<NodeId<Expr>>),
    /// `new.target` / `import.meta`.
    MetaProperty(MetaPropertyKind),
    /// Parenthesized expression (for span tracking / cover grammar).
    Paren(NodeId<Expr>),
    /// `#field in obj` — private field membership test (ES2022).
    PrivateIn { name: Atom, right: NodeId<Expr> },

    /// Error recovery placeholder.
    Error,
}

// ── Expression sub-types ─────────────────────────────────────────────

/// The two valid meta-property forms.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MetaPropertyKind {
    /// `new.target`
    NewTarget,
    /// `import.meta`
    ImportMeta,
}

#[derive(Debug, Clone)]
pub enum Literal {
    Number(f64),
    BigInt(Atom),
    String(Atom),
    Boolean(bool),
    Null,
    RegExp { pattern: Atom, flags: Atom },
}

#[derive(Debug, Clone)]
pub enum ArrayElement {
    Expression(NodeId<Expr>),
    Spread(NodeId<Expr>),
}

/// Bitfield flags for `Property`.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct PropertyFlags(u8);

impl PropertyFlags {
    const COMPUTED: u8 = 1;
    const SHORTHAND: u8 = 2;
    const METHOD: u8 = 4;
    const SPREAD: u8 = 8;

    #[must_use]
    pub fn computed(self) -> bool {
        self.0 & Self::COMPUTED != 0
    }
    #[must_use]
    pub fn shorthand(self) -> bool {
        self.0 & Self::SHORTHAND != 0
    }
    #[must_use]
    pub fn method(self) -> bool {
        self.0 & Self::METHOD != 0
    }
    #[must_use]
    pub fn is_spread(self) -> bool {
        self.0 & Self::SPREAD != 0
    }

    pub fn set_computed(&mut self) {
        self.0 |= Self::COMPUTED;
    }
    pub fn set_shorthand(&mut self) {
        self.0 |= Self::SHORTHAND;
    }
    pub fn set_method(&mut self) {
        self.0 |= Self::METHOD;
    }
    pub fn set_spread(&mut self) {
        self.0 |= Self::SPREAD;
    }
}

#[derive(Debug, Clone)]
pub struct Property {
    pub kind: PropertyKind,
    pub key: PropertyKey,
    pub value: Option<NodeId<Expr>>,
    pub flags: PropertyFlags,
    pub span: Span,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PropertyKind {
    Init,
    Get,
    Set,
}

#[derive(Debug, Clone)]
pub enum PropertyKey {
    Identifier(Atom),
    Literal(Literal),
    Computed(NodeId<Expr>),
    PrivateIdentifier(Atom),
}

#[derive(Debug, Clone)]
pub struct TemplateLiteral {
    pub quasis: Vec<TemplateElement>,
    pub expressions: Vec<NodeId<Expr>>,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct TemplateElement {
    pub raw: Atom,
    pub cooked: Option<Atom>,
    pub tail: bool,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub enum MemberProp {
    Identifier(Atom),
    PrivateIdentifier(Atom),
    Expression(NodeId<Expr>),
}

#[derive(Debug, Clone)]
pub enum OptionalChainPart {
    Member {
        property: MemberProp,
        computed: bool,
    },
    Call(Vec<Argument>),
}

#[derive(Debug, Clone)]
pub enum Argument {
    Expression(NodeId<Expr>),
    Spread(NodeId<Expr>),
}

#[derive(Debug, Clone)]
pub enum AssignTarget {
    /// Simple assignment target (identifier or member expression).
    Simple(NodeId<Expr>),
    /// Destructuring pattern.
    Pattern(NodeId<Pattern>),
}

// ── Operators ────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnaryOp {
    Minus,
    Plus,
    Not,
    BitwiseNot,
    Typeof,
    Void,
    Delete,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UpdateOp {
    Increment,
    Decrement,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BinaryOp {
    Add,
    Sub,
    Mul,
    Div,
    Mod,
    Exp,
    Shl,
    Shr,
    UShr,
    BitAnd,
    BitOr,
    BitXor,
    Eq,
    NotEq,
    StrictEq,
    StrictNotEq,
    Lt,
    LtEq,
    Gt,
    GtEq,
    In,
    Instanceof,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LogicalOp {
    And,
    Or,
    NullCoal,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AssignOp {
    Assign,
    AddAssign,
    SubAssign,
    MulAssign,
    DivAssign,
    ModAssign,
    ExpAssign,
    ShlAssign,
    ShrAssign,
    UShrAssign,
    BitAndAssign,
    BitOrAssign,
    BitXorAssign,
    AndAssign,
    OrAssign,
    NullCoalAssign,
}

// ── Functions & Classes ──────────────────────────────────────────────

/// Shared function structure (declaration, expression, method).
#[derive(Debug, Clone)]
pub struct Function {
    pub name: Option<Atom>,
    pub params: Vec<Param>,
    pub body: Vec<NodeId<Stmt>>,
    pub is_async: bool,
    pub is_generator: bool,
    pub span: Span,
}

/// Function parameter.
#[derive(Debug, Clone)]
pub struct Param {
    pub pattern: NodeId<Pattern>,
    pub default: Option<NodeId<Expr>>,
    pub rest: bool,
    pub span: Span,
}

impl Param {
    /// R12: Shorthand for a simple parameter (no default, not rest).
    #[must_use]
    pub fn simple(pattern: NodeId<Pattern>, span: Span) -> Self {
        Self {
            pattern,
            default: None,
            rest: false,
            span,
        }
    }
}

/// Arrow function.
#[derive(Debug, Clone)]
pub struct ArrowFunction {
    pub params: Vec<Param>,
    pub body: ArrowBody,
    pub is_async: bool,
    pub span: Span,
}

/// Arrow body: expression or block.
#[derive(Debug, Clone)]
pub enum ArrowBody {
    Expression(NodeId<Expr>),
    Block(Vec<NodeId<Stmt>>),
}

/// Shared class structure.
#[derive(Debug, Clone)]
pub struct Class {
    pub name: Option<Atom>,
    pub super_class: Option<NodeId<Expr>>,
    pub body: Vec<ClassMember>,
    pub span: Span,
}

/// Class member.
#[derive(Debug, Clone)]
pub struct ClassMember {
    pub kind: ClassMemberKind,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub enum ClassMemberKind {
    /// Method (including constructor, getter, setter).
    Method {
        key: PropertyKey,
        function: Function,
        kind: MethodKind,
        is_static: bool,
        computed: bool,
    },
    /// Public field: `key = value`.
    Property {
        key: PropertyKey,
        value: Option<NodeId<Expr>>,
        is_static: bool,
        computed: bool,
    },
    /// Private field: `#field = value` (ES2022).
    PrivateField {
        name: Atom,
        value: Option<NodeId<Expr>>,
        is_static: bool,
    },
    /// Private method (ES2022).
    PrivateMethod {
        name: Atom,
        function: Function,
        kind: MethodKind,
        is_static: bool,
    },
    /// `static { ... }` (ES2022).
    StaticBlock(Vec<NodeId<Stmt>>),
    /// Empty slot (from `;`).
    Empty,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum MethodKind {
    #[default]
    Method,
    Constructor,
    Get,
    Set,
}

// ── Patterns (destructuring) ─────────────────────────────────────────

/// A destructuring / binding pattern.
#[derive(Debug, Clone)]
pub struct Pattern {
    pub kind: PatternKind,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub enum PatternKind {
    /// Simple identifier binding.
    Identifier(Atom),
    /// Array pattern `[a, , b, ...rest]`.
    Array {
        elements: Vec<Option<ArrayPatternElement>>,
        rest: Option<NodeId<Pattern>>,
    },
    /// Object pattern `{ a, b: c, ...rest }`.
    Object {
        properties: Vec<ObjectPatternProp>,
        rest: Option<NodeId<Pattern>>,
    },
    /// Default value: `pattern = expr`.
    Assign {
        left: NodeId<Pattern>,
        right: NodeId<Expr>,
    },
    /// An expression that will be validated as assignment target.
    Expression(NodeId<Expr>),

    /// Error recovery placeholder.
    Error,
}

#[derive(Debug, Clone)]
pub struct ArrayPatternElement {
    pub pattern: NodeId<Pattern>,
    pub default: Option<NodeId<Expr>>,
}

#[derive(Debug, Clone)]
pub struct ObjectPatternProp {
    pub key: PropertyKey,
    pub value: NodeId<Pattern>,
    pub computed: bool,
    pub shorthand: bool,
    pub span: Span,
}

// ── Module declarations ──────────────────────────────────────────────

/// Import declaration.
#[derive(Debug, Clone)]
pub struct ImportDecl {
    pub specifiers: Vec<ImportSpecifier>,
    pub source: Atom,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub enum ImportSpecifier {
    /// `import x from 'mod'`
    Default(Atom, Span),
    /// `import * as x from 'mod'`
    Namespace(Atom, Span),
    /// `import { a as b } from 'mod'`
    Named {
        imported: Atom,
        local: Atom,
        span: Span,
    },
}

/// Export declaration.
#[derive(Debug, Clone)]
pub struct ExportDecl {
    pub kind: ExportKind,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub enum ExportKind {
    /// `export { a, b as c }`
    Named {
        specifiers: Vec<ExportSpecifier>,
        source: Option<Atom>,
    },
    /// `export default expr`
    Default(NodeId<Expr>),
    /// `export default function name() {}`
    DefaultFunction(Function),
    /// `export default class Name {}`
    DefaultClass(Class),
    /// `export var/let/const ...`
    Declaration(NodeId<Stmt>),
    /// `export * from 'mod'`
    AllFrom { source: Atom },
    /// `export * as ns from 'mod'`
    NamespaceFrom { exported: Atom, source: Atom },
}

#[derive(Debug, Clone)]
pub struct ExportSpecifier {
    pub local: Atom,
    pub exported: Atom,
    pub span: Span,
}
