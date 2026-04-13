# Moli Architecture

## Overview

Moli is a statically typed, container-scoped programming language compiled to bytecode and executed on a custom register-based virtual machine. The entire toolchain is implemented in Rust with zero external VM or GC dependencies.

## Pipeline

```text
Source (.moli) → Lexer → Parser → AST → Semantic Analyzer → Bytecode IR → .mbc Binary → VM → Execution
```

## Compilation Stages

### 1. Lexer (`src/lexer.rs`)

- Built on the `logos` crate for high-performance tokenization
- Produces tokens with precise byte-offset spans for downstream diagnostics
- Skips whitespace, newlines, and comments from the output stream
- Error recovery: collects all invalid tokens before reporting

### 2. Parser (`src/parser.rs`)

- Recursive descent parser consuming the token stream
- Produces a strongly typed AST (defined in `src/ast.rs`)
- Operator precedence via precedence climbing: `||` < `&&` < `==`/`!=` < `<`/`>`/`<=`/`>=` < `+`/`-` < `*`/`/`/`%` < unary
- Error recovery with synchronization at module/container/brace boundaries

### 3. AST (`src/ast.rs`)

- Every node carries a `Span` for error reporting
- All declarations carry `Visibility` metadata (`pub`/`priv`)
- Node types: `Program`, `ModDecl`, `ContainerDecl`, `FuncDecl`, `VarDecl`, `Stmt`, `Expr`
- Expressions include literals, identifiers, binary/unary ops, function calls, field access

### 4. Semantic Analyzer (`src/sema.rs`)

- Hierarchical symbol table with lexical scope resolution (scope stack with parent pointers)
- Two-pass analysis: first registers all modules/containers, then analyzes bodies
- Type inference engine with explicit fallback for annotated types
- Visibility enforcement at container boundaries
- Validates `start` directive: target module must have a `pub container` with `run()` or `main()`
- Intrinsic functions (`print`, `input`, `sqrt`, etc.) pre-registered in global scope

### 5. Bytecode Compiler (`src/bytecode.rs`)

- Lowers AST to a flat instruction stream with a constant pool
- Fixed-width instructions: `[opcode: u8][a: u16][b: u16][c: u16]` = 7 bytes each
- 27 opcodes covering arithmetic, comparison, logic, control flow, I/O, and variable binding
- Linear register allocation (16 registers, wrapping)
- Label resolution for jumps (patch-based: emit placeholder, patch target after block)
- Binary serialization to `.mbc` format with magic header `MOLI`, version byte, constant pool, name table, function table, instruction stream

### 6. Virtual Machine (`src/vm.rs`)

- 16 general-purpose registers holding `Value` variants (Int, Float, Str, Bool, Void)
- Fetch-decode-execute loop with deterministic step counting
- Call stack with frame pointers: each `CALL` saves registers and locals, `RET` restores them
- Variable binding via name-indexed global map per frame
- Region-based memory allocator: push region on `CALL`, pop (bulk free) on `RET`
- Safety valve: configurable max step count to prevent infinite loops
- Division by zero detection at runtime

## Memory Model

- **No garbage collector.** Memory is managed via region-based allocation.
- Each container scope / function call creates a region. All allocations within that region are bulk-freed when the scope exits.
- Variables are immutable by default. Explicit `mut` required for reassignment.
- Values are stack-allocated (passed by value via register copying).

## Instruction Set (v0.1)

| Opcode | Args | Description |
| --- | --- | --- |
| `LOAD_CONST` | dest, idx | Load constant into register |
| `STORE` | dest, src | Copy register |
| `ADD/SUB/MUL/DIV/MOD` | dest, a, b | Arithmetic |
| `NEG/NOT` | dest, src | Unary operators |
| `CMP_EQ/NEQ/LT/GT/LE/GE` | dest, a, b | Comparison |
| `AND/OR` | dest, a, b | Logical operators |
| `JMP` | -, target | Unconditional jump |
| `JZ` | reg, target | Jump if zero/false |
| `CALL` | func_idx, argc | Function call |
| `RET` | - | Return from function |
| `PRINT` | reg, newline | Print register to stdout |
| `INPUT` | dest | Read line from stdin |
| `BIND` | name_idx, reg | Bind register to named variable |
| `LOAD` | name_idx, dest | Load named variable into register |
| `HALT` | - | Stop execution |

## Error Handling

- All pipeline stages return `Result` types — no panics
- Errors carry source spans for precise line/column reporting
- Colorized diagnostic output with source context and caret underlines
- Lexer and parser collect multiple errors before aborting
- CLI returns proper exit codes: 0 (success), 1 (compile error), 2 (runtime error)

## Dependencies

| Crate | Purpose |
| --- | --- |
| `logos` | High-performance lexer generator |
| `clap` | CLI argument parsing with derive macros |
| `thiserror` | Ergonomic error type definitions |
| `anyhow` | Contextual error handling |

## Design Decisions

- **Container-scoped**: Containers are the primary organizational unit, similar to classes but without inheritance. They group related functions and state.
- **No GC**: Region-based cleanup avoids GC pauses and complexity. Each scope has a region; exiting the scope frees everything.
- **Register VM over stack VM**: Registers reduce instruction count and allow more efficient operand addressing. 16 registers are sufficient for v0.1.
- **Fixed-width instructions**: Simplifies fetch-decode and binary serialization. All instructions are 7 bytes.
- **Logos for lexing**: Compile-time generated DFA is faster than hand-written lexers for the token complexity we need.
