// ARCH: Recursive descent parser producing a strongly typed AST.
// Consumes token stream from the lexer. Reports errors with spans for diagnostics.
// Grammar is LL(1) with limited lookahead for binary operator precedence.

use crate::ast::*;
use crate::lexer::{Token, TokenKind};
use crate::utils::{MoliError, Span};

/// Parser state: token stream + cursor
struct Parser {
    tokens: Vec<Token>,
    pos: usize,
    source: String,
}

/// Parse a token stream into a Program AST
pub fn parse(tokens: Vec<Token>, source: &str) -> Result<Program, Vec<MoliError>> {
    let mut parser = Parser {
        tokens,
        pos: 0,
        source: source.to_string(),
    };
    parser.parse_program()
}

impl Parser {
    // --- Token stream helpers ---

    fn peek(&self) -> &Token {
        self.tokens.get(self.pos).unwrap_or_else(|| self.tokens.last().unwrap())
    }

    fn peek_kind(&self) -> &TokenKind {
        &self.peek().kind
    }

    fn advance(&mut self) -> Token {
        let tok = self.tokens[self.pos].clone();
        if self.pos < self.tokens.len() - 1 {
            self.pos += 1;
        }
        tok
    }

    fn is_at_end(&self) -> bool {
        self.pos >= self.tokens.len() - 1
    }

    fn expect(&mut self, kind: &TokenKind) -> Result<Token, MoliError> {
        if std::mem::discriminant(self.peek_kind()) == std::mem::discriminant(kind) {
            Ok(self.advance())
        } else {
            Err(MoliError::new(
                format!("expected {:?}, found '{}'", kind, self.peek().text),
                self.peek().span,
            ))
        }
    }

    fn expect_ident(&mut self) -> Result<(String, Span), MoliError> {
        let tok = self.peek().clone();
        if tok.kind == TokenKind::Ident && !tok.text.is_empty() {
            self.advance();
            Ok((tok.text, tok.span))
        } else {
            Err(MoliError::new(
                format!("expected identifier, found '{}'", tok.text),
                tok.span,
            ))
        }
    }

    fn check(&self, kind: &TokenKind) -> bool {
        std::mem::discriminant(self.peek_kind()) == std::mem::discriminant(kind)
    }

    fn match_token(&mut self, kind: &TokenKind) -> Option<Token> {
        if self.check(kind) {
            Some(self.advance())
        } else {
            None
        }
    }

    // --- Top-level parsing ---

    fn parse_program(&mut self) -> Result<Program, Vec<MoliError>> {
        let mut modules = Vec::new();
        let mut imports = Vec::new();
        let mut start = None;
        let mut errors = Vec::new();

        while !self.is_at_end() {
            match self.peek_kind().clone() {
                TokenKind::Import => {
                    match self.parse_import() {
                        Ok(imp) => imports.push(imp),
                        Err(e) => {
                            errors.push(e);
                            self.advance();
                        }
                    }
                }
                TokenKind::Start => {
                    match self.parse_start() {
                        Ok(s) => start = Some(s),
                        Err(e) => {
                            errors.push(e);
                            self.advance();
                        }
                    }
                }
                TokenKind::Pub | TokenKind::Priv | TokenKind::Mod => {
                    match self.parse_mod_decl() {
                        Ok(m) => modules.push(m),
                        Err(e) => {
                            errors.push(e);
                            self.synchronize();
                        }
                    }
                }
                _ => {
                    errors.push(MoliError::new(
                        format!("unexpected token '{}' at top level", self.peek().text),
                        self.peek().span,
                    ));
                    self.advance();
                }
            }
        }

        if errors.is_empty() {
            Ok(Program { modules, imports, start })
        } else {
            Err(errors)
        }
    }

    /// Error recovery: skip tokens until we find a synchronization point
    fn synchronize(&mut self) {
        while !self.is_at_end() {
            match self.peek_kind() {
                TokenKind::Pub | TokenKind::Priv | TokenKind::Mod
                | TokenKind::Start | TokenKind::Import => return,
                TokenKind::RBrace => {
                    self.advance();
                    return;
                }
                _ => {
                    self.advance();
                }
            }
        }
    }

    fn parse_import(&mut self) -> Result<ImportDecl, MoliError> {
        let start_tok = self.expect(&TokenKind::Import)?;
        let (name, name_span) = self.expect_ident()?;
        Ok(ImportDecl {
            name,
            span: start_tok.span.merge(name_span),
        })
    }

    fn parse_start(&mut self) -> Result<StartDirective, MoliError> {
        let start_tok = self.expect(&TokenKind::Start)?;
        let (module_name, name_span) = self.expect_ident()?;
        Ok(StartDirective {
            module_name,
            span: start_tok.span.merge(name_span),
        })
    }

    fn parse_visibility(&mut self) -> Visibility {
        match self.peek_kind() {
            TokenKind::Pub => {
                self.advance();
                Visibility::Pub
            }
            TokenKind::Priv => {
                self.advance();
                Visibility::Priv
            }
            _ => Visibility::Priv, // ARCH: default visibility is private
        }
    }

    fn parse_mod_decl(&mut self) -> Result<ModDecl, MoliError> {
        let start_span = self.peek().span;
        let visibility = self.parse_visibility();
        self.expect(&TokenKind::Mod)?;
        let (name, _) = self.expect_ident()?;
        self.expect(&TokenKind::LBrace)?;

        let mut containers = Vec::new();
        while !self.check(&TokenKind::RBrace) && !self.is_at_end() {
            containers.push(self.parse_container_decl()?);
        }

        let end_tok = self.expect(&TokenKind::RBrace)?;
        Ok(ModDecl {
            visibility,
            name,
            containers,
            span: start_span.merge(end_tok.span),
        })
    }

    fn parse_container_decl(&mut self) -> Result<ContainerDecl, MoliError> {
        let start_span = self.peek().span;
        let visibility = self.parse_visibility();
        self.expect(&TokenKind::Container)?;
        let (name, _) = self.expect_ident()?;
        self.expect(&TokenKind::LBrace)?;

        let mut functions = Vec::new();
        let mut fields = Vec::new();

        while !self.check(&TokenKind::RBrace) && !self.is_at_end() {
            match self.peek_kind().clone() {
                TokenKind::Func => {
                    functions.push(self.parse_func_decl(Visibility::Priv)?);
                }
                TokenKind::Pub | TokenKind::Priv => {
                    // ARCH: Lookahead to determine if this is a func or field
                    let vis = self.parse_visibility();
                    if self.check(&TokenKind::Func) {
                        functions.push(self.parse_func_decl(vis)?);
                    } else if self.check(&TokenKind::Let) {
                        fields.push(self.parse_var_decl()?);
                    } else {
                        return Err(MoliError::new(
                            format!("expected 'func' or 'let' in container, found '{}'", self.peek().text),
                            self.peek().span,
                        ));
                    }
                }
                TokenKind::Let => {
                    fields.push(self.parse_var_decl()?);
                }
                _ => {
                    return Err(MoliError::new(
                        format!("unexpected token '{}' in container body", self.peek().text),
                        self.peek().span,
                    ));
                }
            }
        }

        let end_tok = self.expect(&TokenKind::RBrace)?;
        Ok(ContainerDecl {
            visibility,
            name,
            functions,
            fields,
            span: start_span.merge(end_tok.span),
        })
    }

    fn parse_func_decl(&mut self, visibility: Visibility) -> Result<FuncDecl, MoliError> {
        let start_span = self.peek().span;
        self.expect(&TokenKind::Func)?;
        let (name, _) = self.expect_ident()?;
        self.expect(&TokenKind::LParen)?;

        let mut params = Vec::new();
        if !self.check(&TokenKind::RParen) {
            params.push(self.parse_param()?);
            while self.match_token(&TokenKind::Comma).is_some() {
                params.push(self.parse_param()?);
            }
        }
        self.expect(&TokenKind::RParen)?;

        // ARCH: Optional return type annotation
        let return_type = if self.match_token(&TokenKind::Arrow).is_some() {
            Some(self.parse_type_annotation()?)
        } else {
            None
        };

        let body = self.parse_block()?;

        Ok(FuncDecl {
            visibility,
            name,
            params,
            return_type,
            body: body.clone(),
            span: start_span.merge(body.span),
        })
    }

    fn parse_param(&mut self) -> Result<Param, MoliError> {
        let start_span = self.peek().span;
        let mutable = self.match_token(&TokenKind::Mut).is_some();
        let (name, _) = self.expect_ident()?;
        self.expect(&TokenKind::Colon)?;
        let type_ann = self.parse_type_annotation()?;
        Ok(Param {
            name,
            type_ann,
            mutable,
            span: start_span.merge(self.tokens[self.pos.saturating_sub(1)].span),
        })
    }

    fn parse_type_annotation(&mut self) -> Result<TypeAnnotation, MoliError> {
        match self.peek_kind().clone() {
            TokenKind::TyInt => { self.advance(); Ok(TypeAnnotation::Int) }
            TokenKind::TyFloat => { self.advance(); Ok(TypeAnnotation::Float) }
            TokenKind::TyBool => { self.advance(); Ok(TypeAnnotation::Bool) }
            TokenKind::TyString => { self.advance(); Ok(TypeAnnotation::StringType) }
            TokenKind::TyVoid => { self.advance(); Ok(TypeAnnotation::Void) }
            TokenKind::Ident => {
                let (name, _) = self.expect_ident()?;
                Ok(TypeAnnotation::Named(name))
            }
            _ => Err(MoliError::new(
                format!("expected type, found '{}'", self.peek().text),
                self.peek().span,
            )),
        }
    }

    fn parse_block(&mut self) -> Result<Block, MoliError> {
        let start_tok = self.expect(&TokenKind::LBrace)?;
        let mut stmts = Vec::new();

        while !self.check(&TokenKind::RBrace) && !self.is_at_end() {
            stmts.push(self.parse_stmt()?);
        }

        let end_tok = self.expect(&TokenKind::RBrace)?;
        Ok(Block {
            stmts,
            span: start_tok.span.merge(end_tok.span),
        })
    }

    fn parse_stmt(&mut self) -> Result<Stmt, MoliError> {
        match self.peek_kind().clone() {
            TokenKind::Let => self.parse_var_decl_stmt(),
            TokenKind::Return => self.parse_return_stmt(),
            TokenKind::If => self.parse_if_stmt(),
            TokenKind::While => self.parse_while_stmt(),
            TokenKind::LBrace => {
                let block = self.parse_block()?;
                Ok(Stmt::Block(block))
            }
            _ => self.parse_expr_or_assign_stmt(),
        }
    }

    fn parse_var_decl_stmt(&mut self) -> Result<Stmt, MoliError> {
        let decl = self.parse_var_decl()?;
        Ok(Stmt::VarDecl(decl))
    }

    fn parse_var_decl(&mut self) -> Result<VarDecl, MoliError> {
        let start_span = self.peek().span;
        self.expect(&TokenKind::Let)?;
        let mutable = self.match_token(&TokenKind::Mut).is_some();
        let (name, _) = self.expect_ident()?;

        let type_ann = if self.match_token(&TokenKind::Colon).is_some() {
            Some(self.parse_type_annotation()?)
        } else {
            None
        };

        let initializer = if self.match_token(&TokenKind::Assign).is_some() {
            Some(self.parse_expr()?)
        } else {
            None
        };

        let end_span = self.tokens[self.pos.saturating_sub(1)].span;
        Ok(VarDecl {
            mutable,
            name,
            type_ann,
            initializer,
            span: start_span.merge(end_span),
        })
    }

    fn parse_return_stmt(&mut self) -> Result<Stmt, MoliError> {
        let start_tok = self.expect(&TokenKind::Return)?;
        let value = if !self.check(&TokenKind::RBrace) && !self.is_at_end() {
            // ARCH: Check if there's an expression following return
            match self.peek_kind() {
                TokenKind::RBrace => None,
                _ => Some(self.parse_expr()?),
            }
        } else {
            None
        };
        let end_span = value.as_ref().map(|e| e.span()).unwrap_or(start_tok.span);
        Ok(Stmt::Return(ReturnStmt {
            value,
            span: start_tok.span.merge(end_span),
        }))
    }

    fn parse_if_stmt(&mut self) -> Result<Stmt, MoliError> {
        let start_tok = self.expect(&TokenKind::If)?;
        let condition = self.parse_expr()?;
        let then_block = self.parse_block()?;
        let else_block = if self.match_token(&TokenKind::Else).is_some() {
            Some(self.parse_block()?)
        } else {
            None
        };
        let end_span = else_block
            .as_ref()
            .map(|b| b.span)
            .unwrap_or(then_block.span);
        Ok(Stmt::If(IfStmt {
            condition,
            then_block,
            else_block,
            span: start_tok.span.merge(end_span),
        }))
    }

    fn parse_while_stmt(&mut self) -> Result<Stmt, MoliError> {
        let start_tok = self.expect(&TokenKind::While)?;
        let condition = self.parse_expr()?;
        let body = self.parse_block()?;
        Ok(Stmt::While(WhileStmt {
            condition,
            body: body.clone(),
            span: start_tok.span.merge(body.span),
        }))
    }

    fn parse_expr_or_assign_stmt(&mut self) -> Result<Stmt, MoliError> {
        let expr = self.parse_expr()?;

        // ARCH: Check if this is an assignment: <ident> = <expr>
        if self.match_token(&TokenKind::Assign).is_some() {
            if let Expr::Ident(name, span) = &expr {
                let value = self.parse_expr()?;
                let end_span = value.span();
                return Ok(Stmt::Assign(AssignStmt {
                    target: name.clone(),
                    value,
                    span: span.merge(end_span),
                }));
            } else {
                return Err(MoliError::new(
                    "invalid assignment target",
                    expr.span(),
                ));
            }
        }

        Ok(Stmt::Expr(expr))
    }

    // --- Expression parsing with precedence climbing ---

    fn parse_expr(&mut self) -> Result<Expr, MoliError> {
        self.parse_or_expr()
    }

    fn parse_or_expr(&mut self) -> Result<Expr, MoliError> {
        let mut left = self.parse_and_expr()?;
        while self.check(&TokenKind::Or) {
            self.advance();
            let right = self.parse_and_expr()?;
            let span = left.span().merge(right.span());
            left = Expr::BinaryOp(Box::new(BinaryExpr {
                op: BinOp::Or,
                left,
                right,
                span,
            }));
        }
        Ok(left)
    }

    fn parse_and_expr(&mut self) -> Result<Expr, MoliError> {
        let mut left = self.parse_equality_expr()?;
        while self.check(&TokenKind::And) {
            self.advance();
            let right = self.parse_equality_expr()?;
            let span = left.span().merge(right.span());
            left = Expr::BinaryOp(Box::new(BinaryExpr {
                op: BinOp::And,
                left,
                right,
                span,
            }));
        }
        Ok(left)
    }

    fn parse_equality_expr(&mut self) -> Result<Expr, MoliError> {
        let mut left = self.parse_comparison_expr()?;
        loop {
            let op = match self.peek_kind() {
                TokenKind::Eq => BinOp::Eq,
                TokenKind::Neq => BinOp::Neq,
                _ => break,
            };
            self.advance();
            let right = self.parse_comparison_expr()?;
            let span = left.span().merge(right.span());
            left = Expr::BinaryOp(Box::new(BinaryExpr {
                op,
                left,
                right,
                span,
            }));
        }
        Ok(left)
    }

    fn parse_comparison_expr(&mut self) -> Result<Expr, MoliError> {
        let mut left = self.parse_additive_expr()?;
        loop {
            let op = match self.peek_kind() {
                TokenKind::Lt => BinOp::Lt,
                TokenKind::Gt => BinOp::Gt,
                TokenKind::Le => BinOp::Le,
                TokenKind::Ge => BinOp::Ge,
                _ => break,
            };
            self.advance();
            let right = self.parse_additive_expr()?;
            let span = left.span().merge(right.span());
            left = Expr::BinaryOp(Box::new(BinaryExpr {
                op,
                left,
                right,
                span,
            }));
        }
        Ok(left)
    }

    fn parse_additive_expr(&mut self) -> Result<Expr, MoliError> {
        let mut left = self.parse_multiplicative_expr()?;
        loop {
            let op = match self.peek_kind() {
                TokenKind::Plus => BinOp::Add,
                TokenKind::Minus => BinOp::Sub,
                _ => break,
            };
            self.advance();
            let right = self.parse_multiplicative_expr()?;
            let span = left.span().merge(right.span());
            left = Expr::BinaryOp(Box::new(BinaryExpr {
                op,
                left,
                right,
                span,
            }));
        }
        Ok(left)
    }

    fn parse_multiplicative_expr(&mut self) -> Result<Expr, MoliError> {
        let mut left = self.parse_unary_expr()?;
        loop {
            let op = match self.peek_kind() {
                TokenKind::Star => BinOp::Mul,
                TokenKind::Slash => BinOp::Div,
                TokenKind::Percent => BinOp::Mod,
                _ => break,
            };
            self.advance();
            let right = self.parse_unary_expr()?;
            let span = left.span().merge(right.span());
            left = Expr::BinaryOp(Box::new(BinaryExpr {
                op,
                left,
                right,
                span,
            }));
        }
        Ok(left)
    }

    fn parse_unary_expr(&mut self) -> Result<Expr, MoliError> {
        match self.peek_kind().clone() {
            TokenKind::Minus => {
                let op_tok = self.advance();
                let operand = self.parse_unary_expr()?;
                let span = op_tok.span.merge(operand.span());
                Ok(Expr::UnaryOp(Box::new(UnaryExpr {
                    op: UnaryOp::Neg,
                    operand,
                    span,
                })))
            }
            TokenKind::Not => {
                let op_tok = self.advance();
                let operand = self.parse_unary_expr()?;
                let span = op_tok.span.merge(operand.span());
                Ok(Expr::UnaryOp(Box::new(UnaryExpr {
                    op: UnaryOp::Not,
                    operand,
                    span,
                })))
            }
            _ => self.parse_call_expr(),
        }
    }

    fn parse_call_expr(&mut self) -> Result<Expr, MoliError> {
        let primary = self.parse_primary()?;

        // ARCH: Check for function call syntax: expr(args...)
        if let Expr::Ident(name, span) = &primary {
            if self.check(&TokenKind::LParen) {
                self.advance(); // consume '('
                let mut args = Vec::new();
                if !self.check(&TokenKind::RParen) {
                    args.push(self.parse_expr()?);
                    while self.match_token(&TokenKind::Comma).is_some() {
                        args.push(self.parse_expr()?);
                    }
                }
                let end_tok = self.expect(&TokenKind::RParen)?;
                return Ok(Expr::Call(Box::new(CallExpr {
                    callee: name.clone(),
                    args,
                    span: span.merge(end_tok.span),
                })));
            }
        }

        Ok(primary)
    }

    fn parse_primary(&mut self) -> Result<Expr, MoliError> {
        let tok = self.peek().clone();
        match &tok.kind {
            TokenKind::IntLit(n) => {
                let n = *n;
                self.advance();
                Ok(Expr::IntLit(n, tok.span))
            }
            TokenKind::FloatLit(f) => {
                let f = *f;
                self.advance();
                Ok(Expr::FloatLit(f, tok.span))
            }
            TokenKind::StringLit(s) => {
                let s = s.clone();
                self.advance();
                Ok(Expr::StringLit(s, tok.span))
            }
            TokenKind::True => {
                self.advance();
                Ok(Expr::BoolLit(true, tok.span))
            }
            TokenKind::False => {
                self.advance();
                Ok(Expr::BoolLit(false, tok.span))
            }
            TokenKind::Ident => {
                let name = tok.text.clone();
                self.advance();
                Ok(Expr::Ident(name, tok.span))
            }
            TokenKind::LParen => {
                self.advance();
                let expr = self.parse_expr()?;
                self.expect(&TokenKind::RParen)?;
                Ok(expr)
            }
            _ => Err(MoliError::new(
                format!("expected expression, found '{}'", tok.text),
                tok.span,
            )),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lexer::Lexer;

    fn parse_source(src: &str) -> Result<Program, Vec<MoliError>> {
        let mut lexer = Lexer::new(src, "test.moli");
        let tokens = lexer.tokenize().expect("lexer should succeed");
        parse(tokens, src)
    }

    #[test]
    fn test_parse_hello_world() {
        let src = r#"import stdio

pub mod Example {
    pub container Printing {
        func run() {
            print("Hello, World!")
        }
    }
}

start Example"#;
        let program = parse_source(src).expect("should parse");
        assert_eq!(program.imports.len(), 1);
        assert_eq!(program.imports[0].name, "stdio");
        assert_eq!(program.modules.len(), 1);
        assert_eq!(program.modules[0].name, "Example");
        assert_eq!(program.modules[0].containers.len(), 1);
        assert_eq!(program.modules[0].containers[0].name, "Printing");
        assert_eq!(program.modules[0].containers[0].functions.len(), 1);
        assert_eq!(program.modules[0].containers[0].functions[0].name, "run");
        assert!(program.start.is_some());
        assert_eq!(program.start.unwrap().module_name, "Example");
    }

    #[test]
    fn test_parse_variable_decl() {
        let src = r#"pub mod Main {
    pub container App {
        func run() {
            let x = 42
            let mut y: Int = 10
            let z = x + y
        }
    }
}
start Main"#;
        let program = parse_source(src).expect("should parse");
        let func = &program.modules[0].containers[0].functions[0];
        assert_eq!(func.body.stmts.len(), 3);
    }

    #[test]
    fn test_parse_if_else() {
        let src = r#"pub mod Main {
    pub container App {
        func run() {
            if x > 0 {
                print("positive")
            } else {
                print("non-positive")
            }
        }
    }
}
start Main"#;
        let program = parse_source(src).expect("should parse");
        let stmt = &program.modules[0].containers[0].functions[0].body.stmts[0];
        assert!(matches!(stmt, Stmt::If(_)));
    }

    #[test]
    fn test_parse_while_loop() {
        let src = r#"pub mod Main {
    pub container App {
        func run() {
            while i < 10 {
                print(i)
            }
        }
    }
}
start Main"#;
        let program = parse_source(src).expect("should parse");
        let stmt = &program.modules[0].containers[0].functions[0].body.stmts[0];
        assert!(matches!(stmt, Stmt::While(_)));
    }

    #[test]
    fn test_parse_binary_operators() {
        let src = r#"pub mod Main {
    pub container App {
        func run() {
            let x = 1 + 2 * 3
        }
    }
}
start Main"#;
        let program = parse_source(src).expect("should parse");
        let stmt = &program.modules[0].containers[0].functions[0].body.stmts[0];
        if let Stmt::VarDecl(decl) = stmt {
            assert!(matches!(decl.initializer, Some(Expr::BinaryOp(_))));
        } else {
            panic!("expected VarDecl");
        }
    }

    #[test]
    fn test_parse_error_recovery() {
        let src = r#"pub mod Main {
    pub container App {
        func run() {
            let = 42
        }
    }
}
start Main"#;
        let result = parse_source(src);
        assert!(result.is_err());
    }
}
