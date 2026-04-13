#![allow(dead_code)]
// ARCH: Library crate root — exposes all modules for integration tests and external use.

pub mod lexer;
pub mod parser;
pub mod ast;
pub mod sema;
pub mod bytecode;
pub mod vm;
pub mod stdlib;
pub mod cli;
pub mod utils;
