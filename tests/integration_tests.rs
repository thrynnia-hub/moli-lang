// ARCH: Integration tests covering the full pipeline from source to execution.
// Tests valid scripts, error paths, and edge cases.

/// Helper to run a .moli source string through the full pipeline in-process
fn run_pipeline(src: &str) -> Result<(), String> {
    // Lexer
    let mut lexer = moli::lexer::Lexer::new(src, "test.moli");
    let tokens = lexer.tokenize().map_err(|errs| {
        errs.iter()
            .map(|e| e.message.clone())
            .collect::<Vec<_>>()
            .join("; ")
    })?;

    // Parser
    let program = moli::parser::parse(tokens, src).map_err(|errs| {
        errs.iter()
            .map(|e| e.message.clone())
            .collect::<Vec<_>>()
            .join("; ")
    })?;

    // Semantic analysis
    let mut sema = moli::sema::SemanticAnalyzer::new();
    sema.analyze(&program).map_err(|errs| {
        errs.iter()
            .map(|e| e.message.clone())
            .collect::<Vec<_>>()
            .join("; ")
    })?;

    // Bytecode compilation
    let mut compiler = moli::bytecode::Compiler::new();
    let chunk = compiler.compile(&program)?;

    // VM execution
    let mut vm = moli::vm::VM::new(chunk);
    vm.execute()
}

// --- Valid program tests ---

#[test]
fn test_hello_world() {
    let src = r#"import stdio

pub mod Example {
    pub container Printing {
        func run() {
            print("Hello, World!")
        }
    }
}

start Example"#;
    assert!(run_pipeline(src).is_ok());
}

#[test]
fn test_arithmetic_operations() {
    let src = r#"pub mod Main {
    pub container App {
        func run() {
            let a = 10
            let b = 3
            let sum = a + b
            let diff = a - b
            let prod = a * b
            let quot = a / b
            print(sum)
        }
    }
}
start Main"#;
    assert!(run_pipeline(src).is_ok());
}

#[test]
fn test_boolean_logic() {
    let src = r#"pub mod Main {
    pub container App {
        func run() {
            let x = true
            let y = false
            let z = x && y
            let w = x || y
            print(z)
            print(w)
        }
    }
}
start Main"#;
    assert!(run_pipeline(src).is_ok());
}

#[test]
fn test_if_else() {
    let src = r#"pub mod Main {
    pub container App {
        func run() {
            let x = 10
            if x > 5 {
                print("big")
            } else {
                print("small")
            }
        }
    }
}
start Main"#;
    assert!(run_pipeline(src).is_ok());
}

#[test]
fn test_while_loop() {
    let src = r#"pub mod Main {
    pub container App {
        func run() {
            let mut i = 0
            while i < 5 {
                print(i)
                i = i + 1
            }
        }
    }
}
start Main"#;
    assert!(run_pipeline(src).is_ok());
}

#[test]
fn test_mutable_variables() {
    let src = r#"pub mod Main {
    pub container App {
        func run() {
            let mut x = 0
            x = 42
            print(x)
        }
    }
}
start Main"#;
    assert!(run_pipeline(src).is_ok());
}

#[test]
fn test_string_operations() {
    let src = r#"pub mod Main {
    pub container App {
        func run() {
            let greeting = "Hello"
            let name = "World"
            print(greeting)
            print(name)
        }
    }
}
start Main"#;
    assert!(run_pipeline(src).is_ok());
}

#[test]
fn test_nested_arithmetic() {
    let src = r#"pub mod Main {
    pub container App {
        func run() {
            let x = (1 + 2) * (3 + 4)
            print(x)
        }
    }
}
start Main"#;
    assert!(run_pipeline(src).is_ok());
}

#[test]
fn test_comparison_operators() {
    let src = r#"pub mod Main {
    pub container App {
        func run() {
            let a = 5
            let b = 10
            if a < b {
                print("less")
            }
            if a <= b {
                print("le")
            }
            if b > a {
                print("greater")
            }
            if b >= a {
                print("ge")
            }
        }
    }
}
start Main"#;
    assert!(run_pipeline(src).is_ok());
}

#[test]
fn test_private_module() {
    let src = r#"pub mod Main {
    pub container App {
        func run() {
            print("private module works")
        }
    }
}
start Main"#;
    assert!(run_pipeline(src).is_ok());
}

#[test]
fn test_main_entry_point() {
    let src = r#"pub mod Main {
    pub container App {
        func main() {
            print("using main() entry")
        }
    }
}
start Main"#;
    assert!(run_pipeline(src).is_ok());
}

#[test]
fn test_float_arithmetic() {
    let src = r#"pub mod Main {
    pub container App {
        func run() {
            let x = 3.14
            let y = 2.0
            let z = x + y
            print(z)
        }
    }
}
start Main"#;
    assert!(run_pipeline(src).is_ok());
}

#[test]
fn test_negation() {
    let src = r#"pub mod Main {
    pub container App {
        func run() {
            let x = -42
            print(x)
        }
    }
}
start Main"#;
    assert!(run_pipeline(src).is_ok());
}

#[test]
fn test_boolean_not() {
    let src = r#"pub mod Main {
    pub container App {
        func run() {
            let x = !true
            print(x)
        }
    }
}
start Main"#;
    assert!(run_pipeline(src).is_ok());
}

// --- Error path tests ---

#[test]
fn test_missing_start_directive() {
    let src = r#"pub mod Main {
    pub container App {
        func run() {
            print("no start")
        }
    }
}"#;
    assert!(run_pipeline(src).is_err());
}

#[test]
fn test_undefined_variable() {
    let src = r#"pub mod Main {
    pub container App {
        func run() {
            print(undefined)
        }
    }
}
start Main"#;
    assert!(run_pipeline(src).is_err());
}

#[test]
fn test_immutable_reassign() {
    let src = r#"pub mod Main {
    pub container App {
        func run() {
            let x = 5
            x = 10
        }
    }
}
start Main"#;
    assert!(run_pipeline(src).is_err());
}

#[test]
fn test_no_pub_container() {
    let src = r#"pub mod Main {
    container App {
        func run() {
            print("priv container")
        }
    }
}
start Main"#;
    assert!(run_pipeline(src).is_err());
}

#[test]
fn test_no_entry_func() {
    let src = r#"pub mod Main {
    pub container App {
        func helper() {
            print("no run or main")
        }
    }
}
start Main"#;
    assert!(run_pipeline(src).is_err());
}

#[test]
fn test_invalid_start_module() {
    let src = r#"pub mod Main {
    pub container App {
        func run() {
            print("ok")
        }
    }
}
start NonExistent"#;
    assert!(run_pipeline(src).is_err());
}

#[test]
fn test_syntax_error() {
    let src = r#"pub mod Main {
    pub container App {
        func run() {
            let = invalid
        }
    }
}
start Main"#;
    assert!(run_pipeline(src).is_err());
}

// --- Bytecode serialization test ---

#[test]
fn test_bytecode_round_trip() {
    let src = r#"pub mod Main {
    pub container App {
        func run() {
            let x = 42
            print(x)
        }
    }
}
start Main"#;

    let mut lexer = moli::lexer::Lexer::new(src, "test.moli");
    let tokens = lexer.tokenize().expect("lex");
    let program = moli::parser::parse(tokens, src).expect("parse");
    let mut sema = moli::sema::SemanticAnalyzer::new();
    sema.analyze(&program).expect("sema");
    let mut compiler = moli::bytecode::Compiler::new();
    let chunk = compiler.compile(&program).expect("compile");

    let tmp = "/tmp/moli_integration_test.mbc";
    moli::bytecode::serialize_chunk(&chunk, tmp).expect("serialize");
    let loaded = moli::bytecode::deserialize_chunk(tmp).expect("deserialize");

    assert_eq!(chunk.constants.len(), loaded.constants.len());
    assert_eq!(chunk.instructions.len(), loaded.instructions.len());
    assert_eq!(chunk.functions.len(), loaded.functions.len());
    assert_eq!(chunk.names.len(), loaded.names.len());

    // Execute the deserialized chunk
    let mut vm = moli::vm::VM::new(loaded);
    assert!(vm.execute().is_ok());

    let _ = std::fs::remove_file(tmp);
}
