# Moli — Modern Optimized Logic Instruction

A high-level, statically typed programming language built in Rust. Moli features container-scoped visibility, region-based memory management, and a custom register-based virtual machine.

## Quick Start

```bash
# Build
cargo build --release

# Run a .moli source file
cargo run -- run examples/hello.moli

# Compile to bytecode (.mbc)
cargo run -- compile examples/hello.moli

# Print version
cargo run -- version
```

## Hello World

```moli
import stdio

pub mod Example {
    pub container Printing {
        func run() {
            print("Hello, World!")
        }
    }
}

start Example
```

## Language Features (v0.1)

### Modules & Containers

```moli
pub mod MyModule {
    pub container MyContainer {
        func run() {
            // entry point
        }
    }
}
start MyModule
```

### Variables

```moli
let x = 42              // immutable (default)
let mut counter = 0     // explicitly mutable
let pi: Float = 3.14    // with type annotation
counter = counter + 1   // only mut variables can be reassigned
```

### Types

- `Int` — 64-bit signed integer
- `Float` — 64-bit floating point
- `Bool` — `true` / `false`
- `String` — UTF-8 string
- `Void` — no value

### Operators

- **Arithmetic:** `+`, `-`, `*`, `/`, `%`
- **Comparison:** `==`, `!=`, `<`, `>`, `<=`, `>=`
- **Logical:** `&&`, `||`, `!`
- **Unary:** `-` (negation), `!` (logical not)

### Control Flow

```moli
if x > 5 {
    print("big")
} else {
    print("small")
}

while i < 10 {
    print(i)
    i = i + 1
}
```

### Functions

```moli
pub func add(a: Int, b: Int) -> Int {
    return a + b
}

func greet() {
    print("Hello!")
}
```

### Visibility

- `pub` — public, accessible from outside the module/container
- `priv` — private (default), scoped to the declaring module/container

### Entry Point

The `start <ModuleName>` directive resolves to a `pub container` containing `func run()` or `func main()`.

### Imports

```moli
import stdio    // print, println, input
```

## CLI Reference

| Command | Description |
| --- | --- |
| `moli run <file>` | Run a .moli source file |
| `moli compile <file>` | Compile to .mbc bytecode |
| `moli version` | Print version |
| `--verbose` / `-v` | Enable verbose pipeline output |
| `--output` / `-o` | Specify output path for compile |

## Build & Test

```bash
# Build
cargo build

# Run all tests (unit + integration)
cargo test

# Run with verbose output
cargo run -- run examples/hello.moli --verbose

# Clippy lint check
cargo clippy
```

## Project Structure

```text
moli-lang/
├── Cargo.toml
├── README.md
├── ARCHITECTURE.md
├── LICENSE.md
├── examples/
│   ├── hello.moli
│   ├── arithmetic.moli
│   ├── control_flow.moli
│   └── variables.moli
├── src/
│   ├── main.rs          # CLI entry point
│   ├── lib.rs           # Library crate root
│   ├── cli.rs           # Clap CLI definitions & pipeline dispatch
│   ├── lexer.rs         # Logos-based lexer with span tracking
│   ├── ast.rs           # Strongly typed AST node definitions
│   ├── parser.rs        # Recursive descent parser
│   ├── sema.rs          # Semantic analysis & type checking
│   ├── bytecode.rs      # Bytecode IR, compiler, .mbc serialization
│   ├── vm.rs            # 16-register VM with region allocator
│   ├── stdlib.rs        # Standard library intrinsic definitions
│   └── utils.rs         # Diagnostics, spans, error formatting
└── tests/
    └── integration_tests.rs
```

## Architecture

See [ARCHITECTURE.md](ARCHITECTURE.md) for detailed design documentation.

## v0.1 Limitations

- No closures or first-class functions
- No generics or traits
- No pattern matching (`match`)
- No `Result<T, E>` / `?` operator (planned for v0.2)
- No multi-file compilation (single source file per run)
- No C interop (host binding API is stubbed)
- No string interpolation
- Field access on containers not yet implemented
- Register allocation uses a simple linear strategy (16 registers, wrapping)
- No optimizer pass on bytecode

## License

MIT
