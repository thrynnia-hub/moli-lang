// ARCH: Semantic analysis pass with hierarchical symbol table, visibility enforcement,
// type inference, and entry-point validation. Produces colorized, line-accurate diagnostics.

use crate::ast::*;
use crate::utils::{MoliError, Span};
use std::collections::HashMap;

/// Inferred or declared type for a symbol
#[derive(Debug, Clone, PartialEq, Eq)]
#[allow(clippy::enum_variant_names)]
pub enum Type {
    Int,
    Float,
    Bool,
    StringType,
    Void,
    Unknown,
    Named(String),
}

impl std::fmt::Display for Type {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Type::Int => write!(f, "Int"),
            Type::Float => write!(f, "Float"),
            Type::Bool => write!(f, "Bool"),
            Type::StringType => write!(f, "String"),
            Type::Void => write!(f, "Void"),
            Type::Unknown => write!(f, "<unknown>"),
            Type::Named(n) => write!(f, "{}", n),
        }
    }
}

impl From<&TypeAnnotation> for Type {
    fn from(ann: &TypeAnnotation) -> Self {
        match ann {
            TypeAnnotation::Int => Type::Int,
            TypeAnnotation::Float => Type::Float,
            TypeAnnotation::Bool => Type::Bool,
            TypeAnnotation::StringType => Type::StringType,
            TypeAnnotation::Void => Type::Void,
            TypeAnnotation::Named(n) => Type::Named(n.clone()),
        }
    }
}

/// Symbol entry in the symbol table
#[derive(Debug, Clone)]
pub struct Symbol {
    pub name: String,
    pub ty: Type,
    pub mutable: bool,
    pub visibility: Visibility,
    pub kind: SymbolKind,
}

#[derive(Debug, Clone, PartialEq)]
pub enum SymbolKind {
    Variable,
    Function { param_types: Vec<Type>, return_type: Type },
    Container,
    Module,
    Intrinsic,
}

/// Hierarchical scope for symbol resolution
#[derive(Debug)]
struct Scope {
    symbols: HashMap<String, Symbol>,
    parent: Option<usize>,
}

/// Semantic analyzer with scope stack and error collection
pub struct SemanticAnalyzer {
    scopes: Vec<Scope>,
    current_scope: usize,
    errors: Vec<MoliError>,
}

impl Default for SemanticAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}

impl SemanticAnalyzer {
    pub fn new() -> Self {
        let global_scope = Scope {
            symbols: HashMap::new(),
            parent: None,
        };
        let mut analyzer = Self {
            scopes: vec![global_scope],
            current_scope: 0,
            errors: Vec::new(),
        };
        // ARCH: Register stdlib intrinsics in global scope
        analyzer.register_intrinsics();
        analyzer
    }

    fn register_intrinsics(&mut self) {
        let intrinsics = vec![
            ("print", vec![Type::Unknown], Type::Void),
            ("println", vec![Type::Unknown], Type::Void),
            ("input", vec![], Type::StringType),
            ("sqrt", vec![Type::Float], Type::Float),
            ("to_int", vec![Type::Unknown], Type::Int),
            ("to_float", vec![Type::Unknown], Type::Float),
            ("to_string", vec![Type::Unknown], Type::StringType),
        ];

        for (name, params, ret) in intrinsics {
            self.define_symbol(Symbol {
                name: name.to_string(),
                ty: ret.clone(),
                mutable: false,
                visibility: Visibility::Pub,
                kind: SymbolKind::Function {
                    param_types: params,
                    return_type: ret,
                },
            });
        }
    }

    fn push_scope(&mut self) {
        let new_scope = Scope {
            symbols: HashMap::new(),
            parent: Some(self.current_scope),
        };
        self.scopes.push(new_scope);
        self.current_scope = self.scopes.len() - 1;
    }

    fn pop_scope(&mut self) {
        if let Some(parent) = self.scopes[self.current_scope].parent {
            self.current_scope = parent;
        }
    }

    fn define_symbol(&mut self, symbol: Symbol) {
        self.scopes[self.current_scope]
            .symbols
            .insert(symbol.name.clone(), symbol);
    }

    fn lookup_symbol(&self, name: &str) -> Option<&Symbol> {
        let mut scope_idx = self.current_scope;
        loop {
            if let Some(sym) = self.scopes[scope_idx].symbols.get(name) {
                return Some(sym);
            }
            match self.scopes[scope_idx].parent {
                Some(parent) => scope_idx = parent,
                None => return None,
            }
        }
    }

    fn error(&mut self, message: impl Into<String>, span: Span) {
        self.errors.push(MoliError::new(message, span));
    }

    /// Main entry: analyze a complete program
    pub fn analyze(&mut self, program: &Program) -> Result<(), Vec<MoliError>> {
        // ARCH: First pass — register all modules and containers
        for module in &program.modules {
            self.define_symbol(Symbol {
                name: module.name.clone(),
                ty: Type::Void,
                mutable: false,
                visibility: module.visibility,
                kind: SymbolKind::Module,
            });
        }

        // ARCH: Second pass — analyze module bodies
        for module in &program.modules {
            self.analyze_module(module);
        }

        // ARCH: Validate start directive
        if let Some(start) = &program.start {
            self.validate_start(start, program);
        } else if !program.modules.is_empty() {
            self.error(
                "missing 'start' directive: program needs an entry point",
                Span::new(0, 0),
            );
        }

        if self.errors.is_empty() {
            Ok(())
        } else {
            Err(self.errors.clone())
        }
    }

    fn analyze_module(&mut self, module: &ModDecl) {
        self.push_scope();

        for container in &module.containers {
            self.define_symbol(Symbol {
                name: container.name.clone(),
                ty: Type::Void,
                mutable: false,
                visibility: container.visibility,
                kind: SymbolKind::Container,
            });
        }

        for container in &module.containers {
            self.analyze_container(container);
        }

        self.pop_scope();
    }

    fn analyze_container(&mut self, container: &ContainerDecl) {
        self.push_scope();

        // ARCH: Register all functions first (allows mutual references)
        for func in &container.functions {
            let param_types: Vec<Type> = func
                .params
                .iter()
                .map(|p| Type::from(&p.type_ann))
                .collect();
            let return_type = func
                .return_type
                .as_ref()
                .map(Type::from)
                .unwrap_or(Type::Void);

            self.define_symbol(Symbol {
                name: func.name.clone(),
                ty: return_type.clone(),
                mutable: false,
                visibility: func.visibility,
                kind: SymbolKind::Function {
                    param_types,
                    return_type,
                },
            });
        }

        // ARCH: Register fields
        for field in &container.fields {
            let ty = field
                .type_ann
                .as_ref()
                .map(Type::from)
                .unwrap_or(Type::Unknown);
            self.define_symbol(Symbol {
                name: field.name.clone(),
                ty,
                mutable: field.mutable,
                visibility: Visibility::Priv,
                kind: SymbolKind::Variable,
            });
        }

        for func in &container.functions {
            self.analyze_func(func);
        }

        self.pop_scope();
    }

    fn analyze_func(&mut self, func: &FuncDecl) {
        self.push_scope();

        for param in &func.params {
            self.define_symbol(Symbol {
                name: param.name.clone(),
                ty: Type::from(&param.type_ann),
                mutable: param.mutable,
                visibility: Visibility::Priv,
                kind: SymbolKind::Variable,
            });
        }

        self.analyze_block(&func.body);

        self.pop_scope();
    }

    fn analyze_block(&mut self, block: &Block) {
        for stmt in &block.stmts {
            self.analyze_stmt(stmt);
        }
    }

    fn analyze_stmt(&mut self, stmt: &Stmt) {
        match stmt {
            Stmt::VarDecl(decl) => self.analyze_var_decl(decl),
            Stmt::Assign(assign) => self.analyze_assign(assign),
            Stmt::Expr(expr) => {
                self.infer_type(expr);
            }
            Stmt::Return(ret) => {
                if let Some(val) = &ret.value {
                    self.infer_type(val);
                }
            }
            Stmt::If(if_stmt) => {
                let cond_type = self.infer_type(&if_stmt.condition);
                if cond_type != Type::Bool && cond_type != Type::Unknown {
                    self.error(
                        format!("if condition must be Bool, found {}", cond_type),
                        if_stmt.condition.span(),
                    );
                }
                self.push_scope();
                self.analyze_block(&if_stmt.then_block);
                self.pop_scope();
                if let Some(else_block) = &if_stmt.else_block {
                    self.push_scope();
                    self.analyze_block(else_block);
                    self.pop_scope();
                }
            }
            Stmt::While(while_stmt) => {
                let cond_type = self.infer_type(&while_stmt.condition);
                if cond_type != Type::Bool && cond_type != Type::Unknown {
                    self.error(
                        format!("while condition must be Bool, found {}", cond_type),
                        while_stmt.condition.span(),
                    );
                }
                self.push_scope();
                self.analyze_block(&while_stmt.body);
                self.pop_scope();
            }
            Stmt::Block(block) => {
                self.push_scope();
                self.analyze_block(block);
                self.pop_scope();
            }
        }
    }

    fn analyze_var_decl(&mut self, decl: &VarDecl) {
        let declared_type = decl.type_ann.as_ref().map(Type::from);
        let inferred_type = decl
            .initializer
            .as_ref()
            .map(|e| self.infer_type(e));

        let ty = match (&declared_type, &inferred_type) {
            (Some(d), Some(i)) => {
                // ARCH: Type check — declared type must match inferred type
                if *d != *i && *i != Type::Unknown && *d != Type::Unknown {
                    self.error(
                        format!(
                            "type mismatch: declared '{}' but initializer has type '{}'",
                            d, i
                        ),
                        decl.span,
                    );
                }
                d.clone()
            }
            (Some(d), None) => d.clone(),
            (None, Some(i)) => i.clone(),
            (None, None) => {
                self.error(
                    format!("cannot infer type for '{}': add a type annotation or initializer", decl.name),
                    decl.span,
                );
                Type::Unknown
            }
        };

        self.define_symbol(Symbol {
            name: decl.name.clone(),
            ty,
            mutable: decl.mutable,
            visibility: Visibility::Priv,
            kind: SymbolKind::Variable,
        });
    }

    fn analyze_assign(&mut self, assign: &AssignStmt) {
        match self.lookup_symbol(&assign.target) {
            Some(sym) => {
                if !sym.mutable {
                    self.error(
                        format!("cannot assign to immutable variable '{}'", assign.target),
                        assign.span,
                    );
                }
            }
            None => {
                self.error(
                    format!("undefined variable '{}'", assign.target),
                    assign.span,
                );
            }
        }
        self.infer_type(&assign.value);
    }

    /// Type inference for expressions — returns the inferred type
    fn infer_type(&mut self, expr: &Expr) -> Type {
        match expr {
            Expr::IntLit(_, _) => Type::Int,
            Expr::FloatLit(_, _) => Type::Float,
            Expr::StringLit(_, _) => Type::StringType,
            Expr::BoolLit(_, _) => Type::Bool,
            Expr::Ident(name, span) => {
                match self.lookup_symbol(name) {
                    Some(sym) => sym.ty.clone(),
                    None => {
                        self.error(format!("undefined variable '{}'", name), *span);
                        Type::Unknown
                    }
                }
            }
            Expr::BinaryOp(binop) => {
                let left_ty = self.infer_type(&binop.left);
                let right_ty = self.infer_type(&binop.right);

                match binop.op {
                    BinOp::Add | BinOp::Sub | BinOp::Mul | BinOp::Div | BinOp::Mod => {
                        // ARCH: Arithmetic ops work on Int and Float, with promotion
                        match (&left_ty, &right_ty) {
                            (Type::Int, Type::Int) => Type::Int,
                            (Type::Float, Type::Float) => Type::Float,
                            (Type::Int, Type::Float) | (Type::Float, Type::Int) => Type::Float,
                            (Type::StringType, Type::StringType) if binop.op == BinOp::Add => {
                                Type::StringType
                            }
                            (Type::Unknown, _) | (_, Type::Unknown) => Type::Unknown,
                            _ => {
                                self.error(
                                    format!(
                                        "invalid operand types for '{}': {} and {}",
                                        binop.op, left_ty, right_ty
                                    ),
                                    binop.span,
                                );
                                Type::Unknown
                            }
                        }
                    }
                    BinOp::Eq | BinOp::Neq | BinOp::Lt | BinOp::Gt | BinOp::Le | BinOp::Ge => {
                        Type::Bool
                    }
                    BinOp::And | BinOp::Or => {
                        if left_ty != Type::Bool && left_ty != Type::Unknown {
                            self.error(
                                format!("expected Bool for '{}', found {}", binop.op, left_ty),
                                binop.left.span(),
                            );
                        }
                        if right_ty != Type::Bool && right_ty != Type::Unknown {
                            self.error(
                                format!("expected Bool for '{}', found {}", binop.op, right_ty),
                                binop.right.span(),
                            );
                        }
                        Type::Bool
                    }
                }
            }
            Expr::UnaryOp(unary) => {
                let operand_ty = self.infer_type(&unary.operand);
                match unary.op {
                    UnaryOp::Neg => {
                        match operand_ty {
                            Type::Int => Type::Int,
                            Type::Float => Type::Float,
                            Type::Unknown => Type::Unknown,
                            _ => {
                                self.error(
                                    format!("cannot negate type {}", operand_ty),
                                    unary.span,
                                );
                                Type::Unknown
                            }
                        }
                    }
                    UnaryOp::Not => {
                        if operand_ty != Type::Bool && operand_ty != Type::Unknown {
                            self.error(
                                format!("'!' requires Bool, found {}", operand_ty),
                                unary.span,
                            );
                        }
                        Type::Bool
                    }
                }
            }
            Expr::Call(call) => {
                match self.lookup_symbol(&call.callee).cloned() {
                    Some(sym) => {
                        if let SymbolKind::Function { param_types, return_type } = &sym.kind {
                            // ARCH: Intrinsics with Unknown param accept any type
                            if !param_types.contains(&Type::Unknown)
                                && call.args.len() != param_types.len()
                            {
                                self.error(
                                    format!(
                                        "'{}' expects {} arguments, found {}",
                                        call.callee,
                                        param_types.len(),
                                        call.args.len()
                                    ),
                                    call.span,
                                );
                            }
                            for arg in &call.args {
                                self.infer_type(arg);
                            }
                            return_type.clone()
                        } else {
                            self.error(
                                format!("'{}' is not a function", call.callee),
                                call.span,
                            );
                            Type::Unknown
                        }
                    }
                    None => {
                        self.error(
                            format!("undefined function '{}'", call.callee),
                            call.span,
                        );
                        Type::Unknown
                    }
                }
            }
            Expr::FieldAccess(fa) => {
                self.infer_type(&fa.object);
                Type::Unknown
            }
        }
    }

    /// Validate that the start directive points to a valid module with a pub container
    /// that has a run() or main() function
    fn validate_start(&mut self, start: &StartDirective, program: &Program) {
        let module = program.modules.iter().find(|m| m.name == start.module_name);
        match module {
            None => {
                self.error(
                    format!("start module '{}' not found", start.module_name),
                    start.span,
                );
            }
            Some(module) => {
                // ARCH: Find a pub container with run() or main()
                let entry_container = module.containers.iter().find(|c| {
                    c.visibility == Visibility::Pub
                        && c.functions
                            .iter()
                            .any(|f| f.name == "run" || f.name == "main")
                });
                if entry_container.is_none() {
                    self.error(
                        format!(
                            "module '{}' has no pub container with a 'run()' or 'main()' function",
                            start.module_name
                        ),
                        start.span,
                    );
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lexer::Lexer;
    use crate::parser;

    fn analyze_source(src: &str) -> Result<(), Vec<MoliError>> {
        let mut lexer = Lexer::new(src, "test.moli");
        let tokens = lexer.tokenize().expect("lexer should succeed");
        let program = parser::parse(tokens, src).expect("parser should succeed");
        let mut analyzer = SemanticAnalyzer::new();
        analyzer.analyze(&program)
    }

    #[test]
    fn test_valid_hello_world() {
        let src = r#"import stdio

pub mod Example {
    pub container Printing {
        func run() {
            print("Hello, World!")
        }
    }
}

start Example"#;
        assert!(analyze_source(src).is_ok());
    }

    #[test]
    fn test_missing_start() {
        let src = r#"pub mod Example {
    pub container Printing {
        func run() {
            print("Hello")
        }
    }
}"#;
        assert!(analyze_source(src).is_err());
    }

    #[test]
    fn test_undefined_variable() {
        let src = r#"pub mod Main {
    pub container App {
        func run() {
            print(undefined_var)
        }
    }
}
start Main"#;
        assert!(analyze_source(src).is_err());
    }

    #[test]
    fn test_immutable_assign() {
        let src = r#"pub mod Main {
    pub container App {
        func run() {
            let x = 5
            x = 10
        }
    }
}
start Main"#;
        assert!(analyze_source(src).is_err());
    }

    #[test]
    fn test_mutable_assign() {
        let src = r#"pub mod Main {
    pub container App {
        func run() {
            let mut x = 5
            x = 10
        }
    }
}
start Main"#;
        assert!(analyze_source(src).is_ok());
    }

    #[test]
    fn test_no_entry_in_start_module() {
        let src = r#"pub mod Main {
    pub container App {
        func helper() {
            print("no run or main")
        }
    }
}
start Main"#;
        assert!(analyze_source(src).is_err());
    }
}
