// ARCH: Register-based virtual machine with 16 registers, call stack with frame pointers,
// constant pool, and region-based memory allocator. Fetch-decode-execute loop with
// deterministic step execution. No GC — region cleanup per container scope.

use crate::bytecode::{Chunk, Constant, OpCode};
use std::collections::HashMap;
use std::io::{self, BufRead, Write};

/// Runtime value in the VM
#[derive(Debug, Clone)]
pub enum Value {
    Int(i64),
    Float(f64),
    Str(String),
    Bool(bool),
    Void,
}

impl Value {
    pub fn is_truthy(&self) -> bool {
        match self {
            Value::Bool(b) => *b,
            Value::Int(n) => *n != 0,
            Value::Float(f) => *f != 0.0,
            Value::Str(s) => !s.is_empty(),
            Value::Void => false,
        }
    }

    pub fn type_name(&self) -> &'static str {
        match self {
            Value::Int(_) => "Int",
            Value::Float(_) => "Float",
            Value::Str(_) => "String",
            Value::Bool(_) => "Bool",
            Value::Void => "Void",
        }
    }
}

impl std::fmt::Display for Value {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Value::Int(n) => write!(f, "{}", n),
            Value::Float(n) => write!(f, "{}", n),
            Value::Str(s) => write!(f, "{}", s),
            Value::Bool(b) => write!(f, "{}", b),
            Value::Void => write!(f, "void"),
        }
    }
}

impl From<&Constant> for Value {
    fn from(c: &Constant) -> Self {
        match c {
            Constant::Int(n) => Value::Int(*n),
            Constant::Float(f) => Value::Float(*f),
            Constant::Str(s) => Value::Str(s.clone()),
            Constant::Bool(b) => Value::Bool(*b),
        }
    }
}

/// Call frame for function invocation
#[derive(Debug, Clone)]
struct CallFrame {
    return_ip: usize,
    /// Saved registers
    saved_regs: [Value; 16],
    /// Local variable bindings for this frame
    locals: HashMap<u16, Value>,
}

/// Region allocator stub — tracks allocations per container scope.
/// In v0.1, this is a simple bump allocator that bulk-frees on scope exit.
#[derive(Debug)]
pub struct RegionAllocator {
    regions: Vec<Region>,
}

#[derive(Debug)]
struct Region {
    // ARCH: Track allocation count for diagnostics; actual data lives in Value enum
    allocation_count: usize,
}

impl Default for RegionAllocator {
    fn default() -> Self {
        Self::new()
    }
}

impl RegionAllocator {
    pub fn new() -> Self {
        Self {
            regions: vec![Region { allocation_count: 0 }],
        }
    }

    pub fn push_region(&mut self) {
        self.regions.push(Region { allocation_count: 0 });
    }

    pub fn pop_region(&mut self) {
        // ARCH: Bulk free — all allocations in this region are dropped
        self.regions.pop();
    }

    pub fn track_alloc(&mut self) {
        if let Some(region) = self.regions.last_mut() {
            region.allocation_count += 1;
        }
    }

    pub fn total_allocations(&self) -> usize {
        self.regions.iter().map(|r| r.allocation_count).sum()
    }
}

/// The Moli virtual machine
pub struct VM {
    /// 16 general-purpose registers
    registers: [Value; 16],
    /// Program counter
    pc: usize,
    /// Call stack
    call_stack: Vec<CallFrame>,
    /// The bytecode chunk being executed
    chunk: Chunk,
    /// Named variable bindings (current scope)
    globals: HashMap<u16, Value>,
    /// Region allocator
    allocator: RegionAllocator,
    /// Execution step counter (for deterministic debugging)
    step_count: u64,
    /// Maximum steps before forced halt (safety valve)
    max_steps: u64,
}

impl VM {
    pub fn new(chunk: Chunk) -> Self {
        const VOID: Value = Value::Void;
        Self {
            registers: [VOID; 16],
            pc: 0,
            call_stack: Vec::new(),
            chunk,
            globals: HashMap::new(),
            allocator: RegionAllocator::new(),
            step_count: 0,
            max_steps: 10_000_000,
        }
    }

    /// Execute the loaded chunk from the entry point
    pub fn execute(&mut self) -> Result<(), String> {
        // ARCH: Find the __entry__ pseudo-function and start there
        let entry = self
            .chunk
            .functions
            .iter()
            .find(|f| f.name == "__entry__")
            .ok_or("no __entry__ function found")?;

        self.pc = entry.start_ip;
        self.allocator.push_region();

        loop {
            if self.step_count >= self.max_steps {
                return Err("execution limit exceeded (possible infinite loop)".to_string());
            }

            if self.pc >= self.chunk.instructions.len() {
                return Err(format!("PC {} out of bounds ({})", self.pc, self.chunk.instructions.len()));
            }

            let inst = self.chunk.instructions[self.pc].clone();
            self.step_count += 1;

            match inst.op {
                OpCode::LoadConst => {
                    let dest = inst.a as usize;
                    let const_idx = inst.b as usize;
                    let value = self
                        .chunk
                        .constants
                        .get(const_idx)
                        .ok_or(format!("constant index {} out of bounds", const_idx))?;
                    self.registers[dest] = Value::from(value);
                    self.allocator.track_alloc();
                    self.pc += 1;
                }
                OpCode::Store => {
                    let dest = inst.a as usize;
                    let src = inst.b as usize;
                    self.registers[dest] = self.registers[src].clone();
                    self.pc += 1;
                }
                OpCode::Add => {
                    let dest = inst.a as usize;
                    let a = inst.b as usize;
                    let b = inst.c as usize;
                    self.registers[dest] = self.arith_op(&self.registers[a].clone(), &self.registers[b].clone(), "+")?;
                    self.pc += 1;
                }
                OpCode::Sub => {
                    let dest = inst.a as usize;
                    let a = inst.b as usize;
                    let b = inst.c as usize;
                    self.registers[dest] = self.arith_op(&self.registers[a].clone(), &self.registers[b].clone(), "-")?;
                    self.pc += 1;
                }
                OpCode::Mul => {
                    let dest = inst.a as usize;
                    let a = inst.b as usize;
                    let b = inst.c as usize;
                    self.registers[dest] = self.arith_op(&self.registers[a].clone(), &self.registers[b].clone(), "*")?;
                    self.pc += 1;
                }
                OpCode::Div => {
                    let dest = inst.a as usize;
                    let a = inst.b as usize;
                    let b = inst.c as usize;
                    let divisor = &self.registers[b].clone();
                    // ARCH: Division by zero check
                    match divisor {
                        Value::Int(0) => return Err("division by zero".to_string()),
                        Value::Float(f) if *f == 0.0 => return Err("division by zero".to_string()),
                        _ => {}
                    }
                    self.registers[dest] = self.arith_op(&self.registers[a].clone(), divisor, "/")?;
                    self.pc += 1;
                }
                OpCode::Mod => {
                    let dest = inst.a as usize;
                    let a = inst.b as usize;
                    let b = inst.c as usize;
                    self.registers[dest] = self.arith_op(&self.registers[a].clone(), &self.registers[b].clone(), "%")?;
                    self.pc += 1;
                }
                OpCode::Neg => {
                    let dest = inst.a as usize;
                    let src = inst.b as usize;
                    self.registers[dest] = match &self.registers[src] {
                        Value::Int(n) => Value::Int(-n),
                        Value::Float(f) => Value::Float(-f),
                        other => return Err(format!("cannot negate {}", other.type_name())),
                    };
                    self.pc += 1;
                }
                OpCode::Not => {
                    let dest = inst.a as usize;
                    let src = inst.b as usize;
                    self.registers[dest] = Value::Bool(!self.registers[src].is_truthy());
                    self.pc += 1;
                }
                OpCode::CmpEq => {
                    let dest = inst.a as usize;
                    let a = inst.b as usize;
                    let b = inst.c as usize;
                    self.registers[dest] = Value::Bool(self.values_equal(&self.registers[a], &self.registers[b]));
                    self.pc += 1;
                }
                OpCode::CmpNeq => {
                    let dest = inst.a as usize;
                    let a = inst.b as usize;
                    let b = inst.c as usize;
                    self.registers[dest] = Value::Bool(!self.values_equal(&self.registers[a], &self.registers[b]));
                    self.pc += 1;
                }
                OpCode::CmpLt => {
                    let dest = inst.a as usize;
                    let a = inst.b as usize;
                    let b = inst.c as usize;
                    self.registers[dest] = self.compare_values(&self.registers[a].clone(), &self.registers[b].clone(), "<")?;
                    self.pc += 1;
                }
                OpCode::CmpGt => {
                    let dest = inst.a as usize;
                    let a = inst.b as usize;
                    let b = inst.c as usize;
                    self.registers[dest] = self.compare_values(&self.registers[a].clone(), &self.registers[b].clone(), ">")?;
                    self.pc += 1;
                }
                OpCode::CmpLe => {
                    let dest = inst.a as usize;
                    let a = inst.b as usize;
                    let b = inst.c as usize;
                    self.registers[dest] = self.compare_values(&self.registers[a].clone(), &self.registers[b].clone(), "<=")?;
                    self.pc += 1;
                }
                OpCode::CmpGe => {
                    let dest = inst.a as usize;
                    let a = inst.b as usize;
                    let b = inst.c as usize;
                    self.registers[dest] = self.compare_values(&self.registers[a].clone(), &self.registers[b].clone(), ">=")?;
                    self.pc += 1;
                }
                OpCode::And => {
                    let dest = inst.a as usize;
                    let a = inst.b as usize;
                    let b = inst.c as usize;
                    self.registers[dest] = Value::Bool(
                        self.registers[a].is_truthy() && self.registers[b].is_truthy(),
                    );
                    self.pc += 1;
                }
                OpCode::Or => {
                    let dest = inst.a as usize;
                    let a = inst.b as usize;
                    let b = inst.c as usize;
                    self.registers[dest] = Value::Bool(
                        self.registers[a].is_truthy() || self.registers[b].is_truthy(),
                    );
                    self.pc += 1;
                }
                OpCode::Jmp => {
                    self.pc = inst.b as usize;
                }
                OpCode::Jz => {
                    let reg = inst.a as usize;
                    if !self.registers[reg].is_truthy() {
                        self.pc = inst.b as usize;
                    } else {
                        self.pc += 1;
                    }
                }
                OpCode::Call => {
                    let func_idx = inst.a as usize;
                    let func = self
                        .chunk
                        .functions
                        .get(func_idx)
                        .ok_or(format!("function index {} out of bounds", func_idx))?
                        .clone();

                    // ARCH: Save current frame
                    let frame = CallFrame {
                        return_ip: self.pc + 1,
                        saved_regs: self.registers.clone(),
                        locals: self.globals.clone(),
                    };
                    self.call_stack.push(frame);
                    self.allocator.push_region();
                    self.pc = func.start_ip;
                }
                OpCode::Ret => {
                    self.allocator.pop_region();
                    if let Some(frame) = self.call_stack.pop() {
                        let return_val = self.registers[0].clone();
                        self.registers = frame.saved_regs;
                        self.globals = frame.locals;
                        self.registers[0] = return_val;
                        self.pc = frame.return_ip;
                    } else {
                        // ARCH: Returning from top-level — execution complete
                        return Ok(());
                    }
                }
                OpCode::Print => {
                    let reg = inst.a as usize;
                    let newline = inst.b != 0;
                    let val = &self.registers[reg];
                    if newline {
                        println!("{}", val);
                    } else {
                        print!("{}", val);
                        let _ = io::stdout().flush();
                    }
                    self.pc += 1;
                }
                OpCode::Halt => {
                    self.allocator.pop_region();
                    return Ok(());
                }
                OpCode::Bind => {
                    let name_idx = inst.a;
                    let reg = inst.b as usize;
                    self.globals.insert(name_idx, self.registers[reg].clone());
                    self.pc += 1;
                }
                OpCode::Load => {
                    let name_idx = inst.a;
                    let dest = inst.b as usize;
                    let value = self
                        .globals
                        .get(&name_idx)
                        .cloned()
                        .unwrap_or_else(|| {
                            let _name = self.chunk.names.get(name_idx as usize)
                                .map(|s| s.as_str())
                                .unwrap_or("<unknown>");
                            Value::Void
                        });
                    self.registers[dest] = value;
                    self.pc += 1;
                }
                OpCode::Input => {
                    let dest = inst.a as usize;
                    let mut line = String::new();
                    let _ = io::stdout().flush();
                    io::stdin()
                        .lock()
                        .read_line(&mut line)
                        .map_err(|e| format!("input error: {}", e))?;
                    // ARCH: Trim trailing newline
                    if line.ends_with('\n') {
                        line.pop();
                        if line.ends_with('\r') {
                            line.pop();
                        }
                    }
                    self.registers[dest] = Value::Str(line);
                    self.allocator.track_alloc();
                    self.pc += 1;
                }
                OpCode::Cmp => {
                    // ARCH: Generic compare — not used directly, specific CmpXx opcodes preferred
                    self.pc += 1;
                }
            }
        }
    }

    fn arith_op(&self, left: &Value, right: &Value, op: &str) -> Result<Value, String> {
        match (left, right) {
            (Value::Int(a), Value::Int(b)) => match op {
                "+" => Ok(Value::Int(a.wrapping_add(*b))),
                "-" => Ok(Value::Int(a.wrapping_sub(*b))),
                "*" => Ok(Value::Int(a.wrapping_mul(*b))),
                "/" => Ok(Value::Int(a / b)),
                "%" => Ok(Value::Int(a % b)),
                _ => Err(format!("unknown arithmetic op '{}'", op)),
            },
            (Value::Float(a), Value::Float(b)) => match op {
                "+" => Ok(Value::Float(a + b)),
                "-" => Ok(Value::Float(a - b)),
                "*" => Ok(Value::Float(a * b)),
                "/" => Ok(Value::Float(a / b)),
                "%" => Ok(Value::Float(a % b)),
                _ => Err(format!("unknown arithmetic op '{}'", op)),
            },
            (Value::Int(a), Value::Float(b)) => {
                self.arith_op(&Value::Float(*a as f64), &Value::Float(*b), op)
            }
            (Value::Float(a), Value::Int(b)) => {
                self.arith_op(&Value::Float(*a), &Value::Float(*b as f64), op)
            }
            (Value::Str(a), Value::Str(b)) if op == "+" => {
                Ok(Value::Str(format!("{}{}", a, b)))
            }
            _ => Err(format!(
                "cannot apply '{}' to {} and {}",
                op,
                left.type_name(),
                right.type_name()
            )),
        }
    }

    fn values_equal(&self, a: &Value, b: &Value) -> bool {
        match (a, b) {
            (Value::Int(x), Value::Int(y)) => x == y,
            (Value::Float(x), Value::Float(y)) => x == y,
            (Value::Str(x), Value::Str(y)) => x == y,
            (Value::Bool(x), Value::Bool(y)) => x == y,
            (Value::Void, Value::Void) => true,
            _ => false,
        }
    }

    fn compare_values(&self, a: &Value, b: &Value, op: &str) -> Result<Value, String> {
        match (a, b) {
            (Value::Int(x), Value::Int(y)) => Ok(Value::Bool(match op {
                "<" => x < y,
                ">" => x > y,
                "<=" => x <= y,
                ">=" => x >= y,
                _ => return Err(format!("unknown compare op '{}'", op)),
            })),
            (Value::Float(x), Value::Float(y)) => Ok(Value::Bool(match op {
                "<" => x < y,
                ">" => x > y,
                "<=" => x <= y,
                ">=" => x >= y,
                _ => return Err(format!("unknown compare op '{}'", op)),
            })),
            (Value::Int(x), Value::Float(y)) => {
                self.compare_values(&Value::Float(*x as f64), &Value::Float(*y), op)
            }
            (Value::Float(x), Value::Int(y)) => {
                self.compare_values(&Value::Float(*x), &Value::Float(*y as f64), op)
            }
            _ => Err(format!(
                "cannot compare {} and {} with '{}'",
                a.type_name(),
                b.type_name(),
                op
            )),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bytecode::Compiler;
    use crate::lexer::Lexer;
    use crate::parser;
    use crate::sema::SemanticAnalyzer;

    fn run_source(src: &str) -> Result<(), String> {
        let mut lexer = Lexer::new(src, "test.moli");
        let tokens = lexer.tokenize().map_err(|e| format!("{:?}", e))?;
        let program = parser::parse(tokens, src).map_err(|e| format!("{:?}", e))?;
        let mut sema = SemanticAnalyzer::new();
        sema.analyze(&program).map_err(|e| format!("{:?}", e))?;
        let mut compiler = Compiler::new();
        let chunk = compiler.compile(&program)?;
        let mut vm = VM::new(chunk);
        vm.execute()
    }

    #[test]
    fn test_hello_world_execution() {
        let src = r#"import stdio

pub mod Example {
    pub container Printing {
        func run() {
            print("Hello, World!")
        }
    }
}

start Example"#;
        assert!(run_source(src).is_ok());
    }

    #[test]
    fn test_arithmetic_execution() {
        let src = r#"pub mod Main {
    pub container App {
        func run() {
            let x = 10 + 20
            print(x)
        }
    }
}
start Main"#;
        assert!(run_source(src).is_ok());
    }

    #[test]
    fn test_variable_binding() {
        let src = r#"pub mod Main {
    pub container App {
        func run() {
            let x = 42
            let y = x
            print(y)
        }
    }
}
start Main"#;
        assert!(run_source(src).is_ok());
    }

    #[test]
    fn test_if_else_execution() {
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
        assert!(run_source(src).is_ok());
    }

    #[test]
    fn test_while_loop_execution() {
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
        assert!(run_source(src).is_ok());
    }

    #[test]
    fn test_region_allocator() {
        let mut alloc = RegionAllocator::new();
        alloc.push_region();
        alloc.track_alloc();
        alloc.track_alloc();
        assert_eq!(alloc.total_allocations(), 2);
        alloc.pop_region();
        assert_eq!(alloc.total_allocations(), 0);
    }
}
