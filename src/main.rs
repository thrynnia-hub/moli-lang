#![allow(dead_code)]
// ARCH: Main entry point for the Moli compiler/runtime CLI.
// Dispatches to subcommands: run, compile, version.

mod lexer;
mod parser;
mod ast;
mod sema;
mod bytecode;
mod vm;
mod stdlib;
mod cli;
mod utils;

use clap::Parser;
use cli::{Cli, Command};

fn main() {
    let cli = Cli::parse();
    let exit_code = match cli.command {
        Command::Run { file, verbose } => cli::run_file(&file, verbose),
        Command::Compile { file, verbose, output } => cli::compile_file(&file, verbose, output),
        Command::Version => {
            println!("moli {}", env!("CARGO_PKG_VERSION"));
            0
        }
    };
    std::process::exit(exit_code);
}
