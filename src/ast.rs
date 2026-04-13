// ARCH: Strongly typed AST nodes for the Moli language.
// Every declaration carries visibility metadata and source spans.
// The AST is the canonical intermediate representation between parsing and semantic analysis.

use crate::utils::Span;

/// Top-level program: a list of modules and a `start` directive
#[derive(Debug, Clone)]
pub struct Program {
    pub modules: Vec<ModDecl>,
    pub imports: Vec<ImportDecl>,
    pub start: Option<StartDirective>,
}

/// `import <module_name>`
#[derive(Debug, Clone)]
pub struct ImportDecl {
    pub name: String,
    pub span: Span,
}

/// `start <ModuleName>` — entry point directive
#[derive(Debug, Clone)]
pub struct StartDirective {
    pub module_name: String,
    pub span: Span,
}

/// Visibility qualifier
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Visibility {
    Pub,
    Priv,
}

/// `[pub|priv] mod <name> { ... }`
#[derive(Debug, Clone)]
pub struct ModDecl {
    pub visibility: Visibility,
    pub name: String,
    pub containers: Vec<ContainerDecl>,
    pub span: Span,
}

/// `[pub|priv] container <name> { ... }`
#[derive(Debug, Clone)]
pub struct ContainerDecl {
    pub visibility: Visibility,
    pub name: String,
    pub functions: Vec<FuncDecl>,
    pub fields: Vec<VarDecl>,
    pub span: Span,
}

/// `[pub|priv] func <name>(<params>) [-> <return_type>] { ... }`
#[derive(Debug, Clone)]
pub struct FuncDecl {
    pub visibility: Visibility,
    pub name: String,
    pub params: Vec<Param>,
    pub return_type: Option<TypeAnnotation>,
    pub body: Block,
    pub span: Span,
}

/// Function parameter
#[derive(Debug, Clone)]
pub struct Param {
    pub name: String,
    pub type_ann: TypeAnnotation,
    pub mutable: bool,
    pub span: Span,
}

/// Type annotation
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TypeAnnotation {
    Int,
    Float,
    Bool,
    StringType,
    Void,
    Named(String),
}

impl std::fmt::Display for TypeAnnotation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TypeAnnotation::Int => write!(f, "Int"),
            TypeAnnotation::Float => write!(f, "Float"),
            TypeAnnotation::Bool => write!(f, "Bool"),
            TypeAnnotation::StringType => write!(f, "String"),
            TypeAnnotation::Void => write!(f, "Void"),
            TypeAnnotation::Named(n) => write!(f, "{}", n),
        }
    }
}

/// Variable declaration: `let [mut] <name> [: <type>] = <expr>`
#[derive(Debug, Clone)]
pub struct VarDecl {
    pub mutable: bool,
    pub name: String,
    pub type_ann: Option<TypeAnnotation>,
    pub initializer: Option<Expr>,
    pub span: Span,
}

/// A block of statements
#[derive(Debug, Clone)]
pub struct Block {
    pub stmts: Vec<Stmt>,
    pub span: Span,
}

/// Statements
#[derive(Debug, Clone)]
pub enum Stmt {
    VarDecl(VarDecl),
    Assign(AssignStmt),
    Expr(Expr),
    Return(ReturnStmt),
    If(IfStmt),
    While(WhileStmt),
    Block(Block),
}

#[derive(Debug, Clone)]
pub struct AssignStmt {
    pub target: String,
    pub value: Expr,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct ReturnStmt {
    pub value: Option<Expr>,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct IfStmt {
    pub condition: Expr,
    pub then_block: Block,
    pub else_block: Option<Block>,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct WhileStmt {
    pub condition: Expr,
    pub body: Block,
    pub span: Span,
}

/// Expressions
#[derive(Debug, Clone)]
pub enum Expr {
    IntLit(i64, Span),
    FloatLit(f64, Span),
    StringLit(String, Span),
    BoolLit(bool, Span),
    Ident(String, Span),
    BinaryOp(Box<BinaryExpr>),
    UnaryOp(Box<UnaryExpr>),
    Call(Box<CallExpr>),
    FieldAccess(Box<FieldAccessExpr>),
}

impl Expr {
    pub fn span(&self) -> Span {
        match self {
            Expr::IntLit(_, s) => *s,
            Expr::FloatLit(_, s) => *s,
            Expr::StringLit(_, s) => *s,
            Expr::BoolLit(_, s) => *s,
            Expr::Ident(_, s) => *s,
            Expr::BinaryOp(b) => b.span,
            Expr::UnaryOp(u) => u.span,
            Expr::Call(c) => c.span,
            Expr::FieldAccess(f) => f.span,
        }
    }
}

#[derive(Debug, Clone)]
pub struct BinaryExpr {
    pub op: BinOp,
    pub left: Expr,
    pub right: Expr,
    pub span: Span,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BinOp {
    Add,
    Sub,
    Mul,
    Div,
    Mod,
    Eq,
    Neq,
    Lt,
    Gt,
    Le,
    Ge,
    And,
    Or,
}

impl std::fmt::Display for BinOp {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BinOp::Add => write!(f, "+"),
            BinOp::Sub => write!(f, "-"),
            BinOp::Mul => write!(f, "*"),
            BinOp::Div => write!(f, "/"),
            BinOp::Mod => write!(f, "%"),
            BinOp::Eq => write!(f, "=="),
            BinOp::Neq => write!(f, "!="),
            BinOp::Lt => write!(f, "<"),
            BinOp::Gt => write!(f, ">"),
            BinOp::Le => write!(f, "<="),
            BinOp::Ge => write!(f, ">="),
            BinOp::And => write!(f, "&&"),
            BinOp::Or => write!(f, "||"),
        }
    }
}

#[derive(Debug, Clone)]
pub struct UnaryExpr {
    pub op: UnaryOp,
    pub operand: Expr,
    pub span: Span,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnaryOp {
    Neg,
    Not,
}

#[derive(Debug, Clone)]
pub struct CallExpr {
    pub callee: String,
    pub args: Vec<Expr>,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct FieldAccessExpr {
    pub object: Expr,
    pub field: String,
    pub span: Span,
}
