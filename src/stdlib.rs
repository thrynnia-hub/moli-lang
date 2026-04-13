// ARCH: Standard library module providing intrinsic bindings for the Moli VM.
// stdio: print, println, input
// math: basic arithmetic is handled by VM opcodes; sqrt and conversions are intrinsics.
// All stdlib functions are registered in the semantic analyzer's global scope
// and compiled as special-cased opcodes in the bytecode compiler.

/// List of all standard library module names recognized by `import`
pub const STDLIB_MODULES: &[&str] = &["stdio", "math"];

/// Check if a module name is a known stdlib module
pub fn is_stdlib_module(name: &str) -> bool {
    STDLIB_MODULES.contains(&name)
}

/// Intrinsic function metadata for the semantic analyzer
#[derive(Debug, Clone)]
pub struct IntrinsicDef {
    pub name: &'static str,
    pub module: &'static str,
    pub param_count: Option<usize>, // None means variadic
    pub description: &'static str,
}

/// All intrinsic function definitions
pub fn intrinsic_definitions() -> Vec<IntrinsicDef> {
    vec![
        IntrinsicDef {
            name: "print",
            module: "stdio",
            param_count: Some(1),
            description: "Print a value to stdout without newline",
        },
        IntrinsicDef {
            name: "println",
            module: "stdio",
            param_count: Some(1),
            description: "Print a value to stdout with newline",
        },
        IntrinsicDef {
            name: "input",
            module: "stdio",
            param_count: Some(0),
            description: "Read a line from stdin",
        },
        IntrinsicDef {
            name: "sqrt",
            module: "math",
            param_count: Some(1),
            description: "Compute square root of a Float",
        },
        IntrinsicDef {
            name: "to_int",
            module: "math",
            param_count: Some(1),
            description: "Convert a value to Int",
        },
        IntrinsicDef {
            name: "to_float",
            module: "math",
            param_count: Some(1),
            description: "Convert a value to Float",
        },
        IntrinsicDef {
            name: "to_string",
            module: "math",
            param_count: Some(1),
            description: "Convert a value to String",
        },
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_stdlib_module_check() {
        assert!(is_stdlib_module("stdio"));
        assert!(is_stdlib_module("math"));
        assert!(!is_stdlib_module("unknown"));
    }

    #[test]
    fn test_intrinsic_definitions() {
        let defs = intrinsic_definitions();
        assert!(defs.iter().any(|d| d.name == "print"));
        assert!(defs.iter().any(|d| d.name == "input"));
        assert!(defs.iter().any(|d| d.name == "sqrt"));
    }
}
