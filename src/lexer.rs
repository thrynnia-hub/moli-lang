// ARCH: Logos-based lexer with precise token spans, line/col tracking, and error recovery.
// Each token carries its span (byte offsets) for downstream diagnostics.

use logos::Logos;
use crate::utils::{Span, MoliError};

/// All token types in the Moli language
#[derive(Logos, Debug, Clone, PartialEq)]
#[logos(skip r"[ \t\r]+")]
pub enum TokenKind {
    // --- Keywords ---
    #[token("pub")]
    Pub,
    #[token("priv")]
    Priv,
    #[token("mod")]
    Mod,
    #[token("container")]
    Container,
    #[token("func")]
    Func,
    #[token("let")]
    Let,
    #[token("mut")]
    Mut,
    #[token("if")]
    If,
    #[token("else")]
    Else,
    #[token("while")]
    While,
    #[token("return")]
    Return,
    #[token("start")]
    Start,
    #[token("import")]
    Import,
    #[token("true")]
    True,
    #[token("false")]
    False,

    // --- Type keywords ---
    #[token("Int")]
    TyInt,
    #[token("Float")]
    TyFloat,
    #[token("Bool")]
    TyBool,
    #[token("String")]
    TyString,
    #[token("Void")]
    TyVoid,

    // --- Literals ---
    #[regex(r"[0-9]+\.[0-9]+", |lex| lex.slice().parse::<f64>().ok())]
    FloatLit(f64),
    #[regex(r"[0-9]+", |lex| lex.slice().parse::<i64>().ok())]
    IntLit(i64),
    #[regex(r#""([^"\\]|\\.)*""#, |lex| {
        let s = lex.slice();
        Some(s[1..s.len()-1].to_string())
    })]
    StringLit(String),

    // --- Identifiers ---
    #[regex(r"[a-zA-Z_][a-zA-Z0-9_]*", priority = 1)]
    Ident,

    // --- Punctuation & Operators ---
    #[token("(")]
    LParen,
    #[token(")")]
    RParen,
    #[token("{")]
    LBrace,
    #[token("}")]
    RBrace,
    #[token("[")]
    LBracket,
    #[token("]")]
    RBracket,
    #[token(",")]
    Comma,
    #[token(":")]
    Colon,
    #[token(";")]
    Semicolon,
    #[token(".")]
    Dot,
    #[token("->")]
    Arrow,
    #[token("?")]
    Question,

    // --- Operators ---
    #[token("+")]
    Plus,
    #[token("-")]
    Minus,
    #[token("*")]
    Star,
    #[token("/")]
    Slash,
    #[token("%")]
    Percent,
    #[token("=")]
    Assign,
    #[token("==")]
    Eq,
    #[token("!=")]
    Neq,
    #[token("<")]
    Lt,
    #[token(">")]
    Gt,
    #[token("<=")]
    Le,
    #[token(">=")]
    Ge,
    #[token("&&")]
    And,
    #[token("||")]
    Or,
    #[token("!")]
    Not,

    // --- Whitespace ---
    #[regex(r"\n")]
    Newline,

    // --- Comments ---
    #[regex(r"//[^\n]*")]
    LineComment,
}

/// A token with its kind, source slice, and byte-offset span
#[derive(Debug, Clone)]
pub struct Token {
    pub kind: TokenKind,
    pub text: String,
    pub span: Span,
}

/// Lexer wrapping logos with error collection
pub struct Lexer<'a> {
    source: &'a str,
    file: &'a str,
}

impl<'a> Lexer<'a> {
    pub fn new(source: &'a str, file: &'a str) -> Self {
        Self { source, file }
    }

    /// Tokenize the entire source, collecting errors for invalid tokens.
    /// Skips newlines and comments from the output stream.
    pub fn tokenize(&mut self) -> Result<Vec<Token>, Vec<MoliError>> {
        let mut tokens = Vec::new();
        let mut errors = Vec::new();
        let lex = TokenKind::lexer(self.source);

        for (result, span) in lex.spanned() {
            match result {
                Ok(kind) => {
                    // ARCH: Skip whitespace tokens and comments; they are not needed downstream
                    match kind {
                        TokenKind::Newline | TokenKind::LineComment => continue,
                        _ => {}
                    }
                    tokens.push(Token {
                        kind,
                        text: self.source[span.clone()].to_string(),
                        span: Span::new(span.start, span.end),
                    });
                }
                Err(()) => {
                    errors.push(MoliError::new(
                        format!(
                            "unexpected character '{}'",
                            &self.source[span.clone()]
                        ),
                        Span::new(span.start, span.end),
                    ));
                }
            }
        }

        // ARCH: Add EOF sentinel token
        let eof_pos = self.source.len();
        tokens.push(Token {
            kind: TokenKind::Ident, // reuse as EOF marker placeholder
            text: String::new(),
            span: Span::new(eof_pos, eof_pos),
        });

        if errors.is_empty() {
            Ok(tokens)
        } else {
            Err(errors)
        }
    }
}

/// Check if an Ident token text is actually a keyword that logos would match.
/// This is a helper for the parser to distinguish identifiers from contextual keywords.
pub fn is_keyword(text: &str) -> bool {
    matches!(
        text,
        "pub" | "priv" | "mod" | "container" | "func" | "let" | "mut"
            | "if" | "else" | "while" | "return" | "start" | "import"
            | "true" | "false" | "Int" | "Float" | "Bool" | "String" | "Void"
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hello_world_tokens() {
        let src = r#"pub mod Example {
    pub container Printing {
        func run() {
            print("Hello, World!")
        }
    }
}
start Example"#;
        let mut lexer = Lexer::new(src, "test.moli");
        let tokens = lexer.tokenize().expect("should lex without errors");
        // Check key tokens exist
        assert!(tokens.iter().any(|t| t.kind == TokenKind::Pub));
        assert!(tokens.iter().any(|t| t.kind == TokenKind::Mod));
        assert!(tokens.iter().any(|t| t.kind == TokenKind::Container));
        assert!(tokens.iter().any(|t| t.kind == TokenKind::Func));
        assert!(tokens.iter().any(|t| t.kind == TokenKind::Start));
        assert!(tokens.iter().any(|t| matches!(t.kind, TokenKind::StringLit(_))));
    }

    #[test]
    fn test_operators() {
        let src = "a + b - c * d / e == f != g < h > i <= j >= k && l || m";
        let mut lexer = Lexer::new(src, "test.moli");
        let tokens = lexer.tokenize().expect("should lex");
        let ops: Vec<&TokenKind> = tokens
            .iter()
            .filter(|t| !matches!(t.kind, TokenKind::Ident) || !t.text.is_empty())
            .map(|t| &t.kind)
            .collect();
        assert!(ops.contains(&&TokenKind::Plus));
        assert!(ops.contains(&&TokenKind::Minus));
        assert!(ops.contains(&&TokenKind::Star));
        assert!(ops.contains(&&TokenKind::Eq));
        assert!(ops.contains(&&TokenKind::Neq));
        assert!(ops.contains(&&TokenKind::And));
        assert!(ops.contains(&&TokenKind::Or));
    }

    #[test]
    fn test_number_literals() {
        let src = "42 3.14";
        let mut lexer = Lexer::new(src, "test.moli");
        let tokens = lexer.tokenize().expect("should lex");
        assert!(tokens.iter().any(|t| matches!(t.kind, TokenKind::IntLit(42))));
        assert!(tokens.iter().any(|t| matches!(t.kind, TokenKind::FloatLit(f) if (f - 3.14).abs() < 0.001)));
    }

    #[test]
    fn test_invalid_token() {
        let src = "let x = @invalid";
        let mut lexer = Lexer::new(src, "test.moli");
        let result = lexer.tokenize();
        assert!(result.is_err());
    }

    #[test]
    fn test_string_literal() {
        let src = r#""hello world""#;
        let mut lexer = Lexer::new(src, "test.moli");
        let tokens = lexer.tokenize().expect("should lex");
        assert!(tokens.iter().any(|t| matches!(&t.kind, TokenKind::StringLit(s) if s == "hello world")));
    }

    #[test]
    fn test_comments_skipped() {
        let src = "// this is a comment\nlet x = 5";
        let mut lexer = Lexer::new(src, "test.moli");
        let tokens = lexer.tokenize().expect("should lex");
        assert!(!tokens.iter().any(|t| t.kind == TokenKind::LineComment));
    }
}
