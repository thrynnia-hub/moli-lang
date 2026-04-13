// ARCH: CLI module using clap derive API. Provides run, compile, and version subcommands.
// All subcommands return integer exit codes for process::exit().

use clap::{Parser, Subcommand};
use std::path::Path;

use crate::lexer::Lexer;
use crate::parser;
use crate::sema::SemanticAnalyzer;
use crate::bytecode::Compiler;
use crate::vm::VM;
use crate::utils::DiagnosticPrinter;

#[derive(Parser)]
#[command(name = "moli", version, about = "Moli language compiler and runtime")]
pub struct Cli {
    #[command(subcommand)]
    pub command: Command,
}

#[derive(Subcommand)]
pub enum Command {
    /// Run a .moli source file
    Run {
        /// Path to the .moli source file
        file: String,
        /// Enable verbose output
        #[arg(short, long, default_value_t = false)]
        verbose: bool,
    },
    /// Compile a .moli source file to bytecode (.mbc)
    Compile {
        /// Path to the .moli source file
        file: String,
        /// Enable verbose output
        #[arg(short, long, default_value_t = false)]
        verbose: bool,
        /// Output file path (default: <input>.mbc)
        #[arg(short, long)]
        output: Option<String>,
    },
    /// Print version information
    Version,
}

/// Run a .moli file end-to-end: lex → parse → analyze → compile → execute
pub fn run_file(file: &str, verbose: bool) -> i32 {
    let source = match std::fs::read_to_string(file) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("\x1b[1;31merror\x1b[0m: cannot read '{}': {}", file, e);
            return 1;
        }
    };

    if verbose {
        eprintln!("[moli] reading {}", file);
    }

    // ARCH: Pipeline stages with early-exit on error
    let tokens = match lex_source(&source, file, verbose) {
        Ok(t) => t,
        Err(code) => return code,
    };

    let program = match parse_tokens(tokens, file, &source, verbose) {
        Ok(p) => p,
        Err(code) => return code,
    };

    let analyzed = match analyze_program(program, file, &source, verbose) {
        Ok(a) => a,
        Err(code) => return code,
    };

    let chunk = match compile_program(analyzed, verbose) {
        Ok(c) => c,
        Err(code) => return code,
    };

    match execute_chunk(chunk, verbose) {
        Ok(_) => 0,
        Err(code) => code,
    }
}

/// Compile a .moli file to .mbc bytecode binary
pub fn compile_file(file: &str, verbose: bool, output: Option<String>) -> i32 {
    let source = match std::fs::read_to_string(file) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("\x1b[1;31merror\x1b[0m: cannot read '{}': {}", file, e);
            return 1;
        }
    };

    if verbose {
        eprintln!("[moli] reading {}", file);
    }

    let tokens = match lex_source(&source, file, verbose) {
        Ok(t) => t,
        Err(code) => return code,
    };

    let program = match parse_tokens(tokens, file, &source, verbose) {
        Ok(p) => p,
        Err(code) => return code,
    };

    let analyzed = match analyze_program(program, file, &source, verbose) {
        Ok(a) => a,
        Err(code) => return code,
    };

    let chunk = match compile_program(analyzed, verbose) {
        Ok(c) => c,
        Err(code) => return code,
    };

    let out_path = output.unwrap_or_else(|| {
        let p = Path::new(file);
        p.with_extension("mbc").to_string_lossy().to_string()
    });

    match crate::bytecode::serialize_chunk(&chunk, &out_path) {
        Ok(_) => {
            if verbose {
                eprintln!("[moli] wrote bytecode to {}", out_path);
            }
            println!("Compiled {} -> {}", file, out_path);
            0
        }
        Err(e) => {
            eprintln!("\x1b[1;31merror\x1b[0m: failed to write bytecode: {}", e);
            1
        }
    }
}

// --- Internal pipeline stages ---

fn lex_source(
    source: &str,
    file: &str,
    verbose: bool,
) -> Result<Vec<crate::lexer::Token>, i32> {
    if verbose {
        eprintln!("[moli] lexing...");
    }
    let mut lexer = Lexer::new(source, file);
    match lexer.tokenize() {
        Ok(tokens) => {
            if verbose {
                eprintln!("[moli] {} tokens produced", tokens.len());
            }
            Ok(tokens)
        }
        Err(errors) => {
            let printer = DiagnosticPrinter::new(source, file);
            for err in &errors {
                printer.print_error(&err.message, err.span.start, err.span.end);
            }
            Err(1)
        }
    }
}

fn parse_tokens(
    tokens: Vec<crate::lexer::Token>,
    file: &str,
    source: &str,
    verbose: bool,
) -> Result<crate::ast::Program, i32> {
    if verbose {
        eprintln!("[moli] parsing...");
    }
    match parser::parse(tokens, source) {
        Ok(program) => {
            if verbose {
                eprintln!("[moli] AST constructed ({} modules)", program.modules.len());
            }
            Ok(program)
        }
        Err(errors) => {
            let printer = DiagnosticPrinter::new(source, file);
            for err in &errors {
                printer.print_error(&err.message, err.span.start, err.span.end);
            }
            Err(1)
        }
    }
}

fn analyze_program(
    program: crate::ast::Program,
    file: &str,
    source: &str,
    verbose: bool,
) -> Result<crate::ast::Program, i32> {
    if verbose {
        eprintln!("[moli] analyzing...");
    }
    let mut analyzer = SemanticAnalyzer::new();
    match analyzer.analyze(&program) {
        Ok(()) => {
            if verbose {
                eprintln!("[moli] semantic analysis passed");
            }
            Ok(program)
        }
        Err(errors) => {
            let printer = DiagnosticPrinter::new(source, file);
            for err in &errors {
                printer.print_error(&err.message, err.span.start, err.span.end);
            }
            Err(1)
        }
    }
}

fn compile_program(
    program: crate::ast::Program,
    verbose: bool,
) -> Result<crate::bytecode::Chunk, i32> {
    if verbose {
        eprintln!("[moli] compiling to bytecode...");
    }
    let mut compiler = Compiler::new();
    match compiler.compile(&program) {
        Ok(chunk) => {
            if verbose {
                eprintln!(
                    "[moli] bytecode: {} instructions, {} constants",
                    chunk.instructions.len(),
                    chunk.constants.len()
                );
            }
            Ok(chunk)
        }
        Err(e) => {
            eprintln!("\x1b[1;31mcompile error\x1b[0m: {}", e);
            Err(1)
        }
    }
}

fn execute_chunk(chunk: crate::bytecode::Chunk, verbose: bool) -> Result<(), i32> {
    if verbose {
        eprintln!("[moli] executing...");
    }
    let mut vm = VM::new(chunk);
    match vm.execute() {
        Ok(()) => {
            if verbose {
                eprintln!("[moli] execution completed successfully");
            }
            Ok(())
        }
        Err(e) => {
            eprintln!("\x1b[1;31mruntime error\x1b[0m: {}", e);
            Err(2)
        }
    }
}
