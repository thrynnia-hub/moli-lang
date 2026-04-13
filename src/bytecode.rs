// ARCH: Bytecode IR with fixed-width instruction set, constant pool, and label resolution.
// AST-to-IR lowering pass with implicit context frame setup and linear register allocation.
// Binary serialization to .mbc format with magic header, version, metadata, instruction stream.

use crate::ast::*;
use std::collections::HashMap;

/// Magic bytes for .mbc binary format
pub const MBC_MAGIC: &[u8; 4] = b"MOLI";
/// Binary format version
pub const MBC_VERSION: u8 = 1;

/// Opcodes for the Moli register VM
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum OpCode {
    /// LOAD_CONST dest, const_idx — load constant pool entry into register
    LoadConst = 0,
    /// STORE dest, src — copy register src to register dest
    Store = 1,
    /// ADD dest, a, b — dest = a + b
    Add = 2,
    /// SUB dest, a, b — dest = a - b
    Sub = 3,
    /// MUL dest, a, b — dest = a * b
    Mul = 4,
    /// DIV dest, a, b — dest = a / b
    Div = 5,
    /// MOD dest, a, b — dest = a % b
    Mod = 6,
    /// CMP dest, a, b — compare a, b; store comparison result in dest
    Cmp = 7,
    /// JMP offset — unconditional jump
    Jmp = 8,
    /// JZ reg, offset — jump if register is zero/false
    Jz = 9,
    /// CALL func_idx, arg_count — call function, push frame
    Call = 10,
    /// RET — return from current frame
    Ret = 11,
    /// PRINT reg — print register value to stdout
    Print = 12,
    /// HALT — stop execution
    Halt = 13,
    /// NEG dest, src — dest = -src
    Neg = 14,
    /// NOT dest, src — dest = !src
    Not = 15,
    /// BIND name_idx, reg — bind register to name in current scope
    Bind = 16,
    /// LOAD name_idx, dest — load named variable into register
    Load = 17,
    /// CMP_EQ dest, a, b
    CmpEq = 18,
    /// CMP_NEQ dest, a, b
    CmpNeq = 19,
    /// CMP_LT dest, a, b
    CmpLt = 20,
    /// CMP_GT dest, a, b
    CmpGt = 21,
    /// CMP_LE dest, a, b
    CmpLe = 22,
    /// CMP_GE dest, a, b
    CmpGe = 23,
    /// AND dest, a, b
    And = 24,
    /// OR dest, a, b
    Or = 25,
    /// INPUT dest — read line from stdin into register
    Input = 26,
}

impl OpCode {
    pub fn from_u8(b: u8) -> Option<Self> {
        match b {
            0 => Some(OpCode::LoadConst),
            1 => Some(OpCode::Store),
            2 => Some(OpCode::Add),
            3 => Some(OpCode::Sub),
            4 => Some(OpCode::Mul),
            5 => Some(OpCode::Div),
            6 => Some(OpCode::Mod),
            7 => Some(OpCode::Cmp),
            8 => Some(OpCode::Jmp),
            9 => Some(OpCode::Jz),
            10 => Some(OpCode::Call),
            11 => Some(OpCode::Ret),
            12 => Some(OpCode::Print),
            13 => Some(OpCode::Halt),
            14 => Some(OpCode::Neg),
            15 => Some(OpCode::Not),
            16 => Some(OpCode::Bind),
            17 => Some(OpCode::Load),
            18 => Some(OpCode::CmpEq),
            19 => Some(OpCode::CmpNeq),
            20 => Some(OpCode::CmpLt),
            21 => Some(OpCode::CmpGt),
            22 => Some(OpCode::CmpLe),
            23 => Some(OpCode::CmpGe),
            24 => Some(OpCode::And),
            25 => Some(OpCode::Or),
            26 => Some(OpCode::Input),
            _ => None,
        }
    }
}

/// A single bytecode instruction with up to 3 operands
#[derive(Debug, Clone)]
pub struct Instruction {
    pub op: OpCode,
    pub a: u16,
    pub b: u16,
    pub c: u16,
}

impl Instruction {
    pub fn new(op: OpCode, a: u16, b: u16, c: u16) -> Self {
        Self { op, a, b, c }
    }

    pub fn simple(op: OpCode) -> Self {
        Self { op, a: 0, b: 0, c: 0 }
    }
}

/// Constant pool value
#[derive(Debug, Clone, PartialEq)]
pub enum Constant {
    Int(i64),
    Float(f64),
    Str(String),
    Bool(bool),
}

impl std::fmt::Display for Constant {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Constant::Int(n) => write!(f, "{}", n),
            Constant::Float(n) => write!(f, "{}", n),
            Constant::Str(s) => write!(f, "{}", s),
            Constant::Bool(b) => write!(f, "{}", b),
        }
    }
}

/// A compiled function
#[derive(Debug, Clone)]
pub struct CompiledFunc {
    pub name: String,
    pub start_ip: usize,
    pub param_count: u16,
}

/// A compiled bytecode chunk — the output of compilation
#[derive(Debug, Clone)]
pub struct Chunk {
    pub instructions: Vec<Instruction>,
    pub constants: Vec<Constant>,
    pub functions: Vec<CompiledFunc>,
    pub entry_func: Option<usize>,
    pub names: Vec<String>,
}

impl Default for Chunk {
    fn default() -> Self {
        Self::new()
    }
}

impl Chunk {
    pub fn new() -> Self {
        Self {
            instructions: Vec::new(),
            constants: Vec::new(),
            functions: Vec::new(),
            entry_func: None,
            names: Vec::new(),
        }
    }

    pub fn add_constant(&mut self, c: Constant) -> u16 {
        // ARCH: Deduplicate constants
        if let Some(idx) = self.constants.iter().position(|existing| existing == &c) {
            return idx as u16;
        }
        let idx = self.constants.len();
        self.constants.push(c);
        idx as u16
    }

    pub fn add_name(&mut self, name: &str) -> u16 {
        if let Some(idx) = self.names.iter().position(|n| n == name) {
            return idx as u16;
        }
        let idx = self.names.len();
        self.names.push(name.to_string());
        idx as u16
    }

    pub fn emit(&mut self, inst: Instruction) -> usize {
        let idx = self.instructions.len();
        self.instructions.push(inst);
        idx
    }

    pub fn current_ip(&self) -> usize {
        self.instructions.len()
    }

    /// Patch a jump instruction's target
    pub fn patch_jump(&mut self, inst_idx: usize, target: u16) {
        self.instructions[inst_idx].b = target;
    }
}

/// Bytecode compiler — lowers AST to bytecode Chunk
pub struct Compiler {
    chunk: Chunk,
    next_reg: u16,
    label_counter: usize,
    // ARCH: Map of function names to their compiled index
    func_map: HashMap<String, usize>,
}

impl Default for Compiler {
    fn default() -> Self {
        Self::new()
    }
}

impl Compiler {
    pub fn new() -> Self {
        Self {
            chunk: Chunk::new(),
            next_reg: 0,
            label_counter: 0,
            func_map: HashMap::new(),
        }
    }

    fn alloc_reg(&mut self) -> u16 {
        let r = self.next_reg;
        self.next_reg += 1;
        // ARCH: Wrap around if we exceed 16 registers (simple strategy for v0.1)
        if self.next_reg >= 16 {
            self.next_reg = 2; // Keep r0, r1 for scratch
        }
        r
    }

    fn reset_regs(&mut self) {
        self.next_reg = 0;
    }

    pub fn compile(&mut self, program: &Program) -> Result<Chunk, String> {
        // ARCH: Find the entry module via start directive
        let start_module_name = program
            .start
            .as_ref()
            .map(|s| s.module_name.clone())
            .ok_or_else(|| "no 'start' directive found. Add 'start <ModuleName>' as the last line of your .moli file".to_string())?;

        let entry_module = program
            .modules
            .iter()
            .find(|m| m.name == start_module_name)
            .ok_or_else(|| format!("start module '{}' not found", start_module_name))?;

        // ARCH: Find the entry container (pub, with run() or main())
        let entry_container = entry_module
            .containers
            .iter()
            .find(|c| {
                c.visibility == Visibility::Pub
                    && c.functions.iter().any(|f| f.name == "run" || f.name == "main")
            })
            .ok_or_else(|| {
                format!(
                    "no pub container with run()/main() in module '{}'",
                    start_module_name
                )
            })?;

        // ARCH: Compile all functions in the entry container
        for func in &entry_container.functions {
            self.compile_func(func)?;
        }

        // ARCH: Emit a CALL to the entry function (run or main), then HALT
        let entry_func_name = entry_container
            .functions
            .iter()
            .find(|f| f.name == "run" || f.name == "main")
            .map(|f| f.name.clone())
            .unwrap();

        let entry_func_idx = *self
            .func_map
            .get(&entry_func_name)
            .ok_or_else(|| format!("entry function '{}' not compiled", entry_func_name))?;

        self.chunk.entry_func = Some(entry_func_idx);

        // ARCH: The VM will start execution at the entry function's start_ip directly
        // We emit a CALL at the beginning and then HALT after
        let call_ip = self.chunk.current_ip();
        self.chunk.emit(Instruction::new(
            OpCode::Call,
            entry_func_idx as u16,
            0, // no args for run()/main()
            0,
        ));
        self.chunk.emit(Instruction::simple(OpCode::Halt));

        // ARCH: Rewrite instruction stream so the preamble (CALL + HALT) is at IP 0.
        // We already emit it at the end, but we need to move it to the front.
        // Instead, we'll note the call_ip and let the VM start there.
        // Actually, let's just record the entry point properly.
        // The VM will start at call_ip.
        self.chunk.functions.push(CompiledFunc {
            name: "__entry__".to_string(),
            start_ip: call_ip,
            param_count: 0,
        });

        Ok(self.chunk.clone())
    }

    fn compile_func(&mut self, func: &FuncDecl) -> Result<(), String> {
        self.reset_regs();
        let start_ip = self.chunk.current_ip();
        let func_idx = self.chunk.functions.len();
        self.func_map.insert(func.name.clone(), func_idx);

        self.chunk.functions.push(CompiledFunc {
            name: func.name.clone(),
            start_ip,
            param_count: func.params.len() as u16,
        });

        // ARCH: Bind parameters to names
        for (i, param) in func.params.iter().enumerate() {
            let name_idx = self.chunk.add_name(&param.name);
            self.chunk.emit(Instruction::new(OpCode::Bind, name_idx, i as u16, 0));
        }

        self.compile_block(&func.body)?;

        // ARCH: Implicit return at end of function
        if self.chunk.instructions.last().map(|i| i.op) != Some(OpCode::Ret) {
            self.chunk.emit(Instruction::simple(OpCode::Ret));
        }

        Ok(())
    }

    fn compile_block(&mut self, block: &Block) -> Result<(), String> {
        for stmt in &block.stmts {
            self.compile_stmt(stmt)?;
        }
        Ok(())
    }

    fn compile_stmt(&mut self, stmt: &Stmt) -> Result<(), String> {
        match stmt {
            Stmt::VarDecl(decl) => self.compile_var_decl(decl),
            Stmt::Assign(assign) => self.compile_assign(assign),
            Stmt::Expr(expr) => {
                self.compile_expr(expr)?;
                Ok(())
            }
            Stmt::Return(ret) => self.compile_return(ret),
            Stmt::If(if_stmt) => self.compile_if(if_stmt),
            Stmt::While(while_stmt) => self.compile_while(while_stmt),
            Stmt::Block(block) => self.compile_block(block),
        }
    }

    fn compile_var_decl(&mut self, decl: &VarDecl) -> Result<(), String> {
        let name_idx = self.chunk.add_name(&decl.name);
        if let Some(init) = &decl.initializer {
            let reg = self.compile_expr(init)?;
            self.chunk.emit(Instruction::new(OpCode::Bind, name_idx, reg, 0));
        }
        Ok(())
    }

    fn compile_assign(&mut self, assign: &AssignStmt) -> Result<(), String> {
        let name_idx = self.chunk.add_name(&assign.target);
        let reg = self.compile_expr(&assign.value)?;
        self.chunk.emit(Instruction::new(OpCode::Bind, name_idx, reg, 0));
        Ok(())
    }

    fn compile_return(&mut self, ret: &ReturnStmt) -> Result<(), String> {
        if let Some(val) = &ret.value {
            let reg = self.compile_expr(val)?;
            self.chunk.emit(Instruction::new(OpCode::Store, 0, reg, 0));
        }
        self.chunk.emit(Instruction::simple(OpCode::Ret));
        Ok(())
    }

    fn compile_if(&mut self, if_stmt: &IfStmt) -> Result<(), String> {
        let cond_reg = self.compile_expr(&if_stmt.condition)?;

        // ARCH: JZ to else branch (or end if no else)
        let jz_ip = self.chunk.emit(Instruction::new(OpCode::Jz, cond_reg, 0, 0));

        self.compile_block(&if_stmt.then_block)?;

        if let Some(else_block) = &if_stmt.else_block {
            // Jump past else block at end of then block
            let jmp_ip = self.chunk.emit(Instruction::new(OpCode::Jmp, 0, 0, 0));
            let else_start = self.chunk.current_ip() as u16;
            self.chunk.patch_jump(jz_ip, else_start);
            self.compile_block(else_block)?;
            let after_else = self.chunk.current_ip() as u16;
            self.chunk.patch_jump(jmp_ip, after_else);
        } else {
            let after_if = self.chunk.current_ip() as u16;
            self.chunk.patch_jump(jz_ip, after_if);
        }

        Ok(())
    }

    fn compile_while(&mut self, while_stmt: &WhileStmt) -> Result<(), String> {
        let loop_start = self.chunk.current_ip() as u16;
        let cond_reg = self.compile_expr(&while_stmt.condition)?;
        let jz_ip = self.chunk.emit(Instruction::new(OpCode::Jz, cond_reg, 0, 0));
        self.compile_block(&while_stmt.body)?;
        self.chunk.emit(Instruction::new(OpCode::Jmp, 0, loop_start, 0));
        let after_loop = self.chunk.current_ip() as u16;
        self.chunk.patch_jump(jz_ip, after_loop);
        Ok(())
    }

    /// Compile an expression, returning the register that holds the result
    fn compile_expr(&mut self, expr: &Expr) -> Result<u16, String> {
        match expr {
            Expr::IntLit(n, _) => {
                let dest = self.alloc_reg();
                let idx = self.chunk.add_constant(Constant::Int(*n));
                self.chunk.emit(Instruction::new(OpCode::LoadConst, dest, idx, 0));
                Ok(dest)
            }
            Expr::FloatLit(f, _) => {
                let dest = self.alloc_reg();
                let idx = self.chunk.add_constant(Constant::Float(*f));
                self.chunk.emit(Instruction::new(OpCode::LoadConst, dest, idx, 0));
                Ok(dest)
            }
            Expr::StringLit(s, _) => {
                let dest = self.alloc_reg();
                let idx = self.chunk.add_constant(Constant::Str(s.clone()));
                self.chunk.emit(Instruction::new(OpCode::LoadConst, dest, idx, 0));
                Ok(dest)
            }
            Expr::BoolLit(b, _) => {
                let dest = self.alloc_reg();
                let idx = self.chunk.add_constant(Constant::Bool(*b));
                self.chunk.emit(Instruction::new(OpCode::LoadConst, dest, idx, 0));
                Ok(dest)
            }
            Expr::Ident(name, _) => {
                let dest = self.alloc_reg();
                let name_idx = self.chunk.add_name(name);
                self.chunk.emit(Instruction::new(OpCode::Load, name_idx, dest, 0));
                Ok(dest)
            }
            Expr::BinaryOp(binop) => {
                let left = self.compile_expr(&binop.left)?;
                let right = self.compile_expr(&binop.right)?;
                let dest = self.alloc_reg();
                let op = match binop.op {
                    BinOp::Add => OpCode::Add,
                    BinOp::Sub => OpCode::Sub,
                    BinOp::Mul => OpCode::Mul,
                    BinOp::Div => OpCode::Div,
                    BinOp::Mod => OpCode::Mod,
                    BinOp::Eq => OpCode::CmpEq,
                    BinOp::Neq => OpCode::CmpNeq,
                    BinOp::Lt => OpCode::CmpLt,
                    BinOp::Gt => OpCode::CmpGt,
                    BinOp::Le => OpCode::CmpLe,
                    BinOp::Ge => OpCode::CmpGe,
                    BinOp::And => OpCode::And,
                    BinOp::Or => OpCode::Or,
                };
                self.chunk.emit(Instruction::new(op, dest, left, right));
                Ok(dest)
            }
            Expr::UnaryOp(unary) => {
                let src = self.compile_expr(&unary.operand)?;
                let dest = self.alloc_reg();
                let op = match unary.op {
                    UnaryOp::Neg => OpCode::Neg,
                    UnaryOp::Not => OpCode::Not,
                };
                self.chunk.emit(Instruction::new(op, dest, src, 0));
                Ok(dest)
            }
            Expr::Call(call) => {
                self.compile_call(call)
            }
            Expr::FieldAccess(_) => {
                Err("field access not yet supported in bytecode".to_string())
            }
        }
    }

    fn compile_call(&mut self, call: &CallExpr) -> Result<u16, String> {
        // ARCH: Handle intrinsic calls specially
        match call.callee.as_str() {
            "print" => {
                for arg in &call.args {
                    let reg = self.compile_expr(arg)?;
                    self.chunk.emit(Instruction::new(OpCode::Print, reg, 0, 0));
                }
                Ok(0) // print returns void
            }
            "println" => {
                for arg in &call.args {
                    let reg = self.compile_expr(arg)?;
                    self.chunk.emit(Instruction::new(OpCode::Print, reg, 1, 0));
                }
                Ok(0)
            }
            "input" => {
                let dest = self.alloc_reg();
                self.chunk.emit(Instruction::new(OpCode::Input, dest, 0, 0));
                Ok(dest)
            }
            _ => {
                // ARCH: User-defined function call
                // Compile arguments
                let mut arg_regs = Vec::new();
                for arg in &call.args {
                    let reg = self.compile_expr(arg)?;
                    arg_regs.push(reg);
                }

                // Look up the function
                if let Some(&func_idx) = self.func_map.get(&call.callee) {
                    self.chunk.emit(Instruction::new(
                        OpCode::Call,
                        func_idx as u16,
                        call.args.len() as u16,
                        0,
                    ));
                    // Return value is in r0 by convention
                    let dest = self.alloc_reg();
                    self.chunk.emit(Instruction::new(OpCode::Store, dest, 0, 0));
                    Ok(dest)
                } else {
                    Err(format!("undefined function '{}' in codegen", call.callee))
                }
            }
        }
    }
}

// --- Binary serialization (.mbc format) ---
// ARCH: Format layout:
// [4 bytes magic "MOLI"]
// [1 byte version]
// [4 bytes num_constants]
//   for each constant:
//     [1 byte type tag] [payload]
// [4 bytes num_names]
//   for each name:
//     [4 bytes length] [utf8 bytes]
// [4 bytes num_functions]
//   for each function:
//     [4 bytes name_len] [name bytes] [4 bytes start_ip] [2 bytes param_count]
// [4 bytes num_instructions]
//   for each instruction:
//     [1 byte opcode] [2 bytes a] [2 bytes b] [2 bytes c]
// [4 bytes entry_func index, or 0xFFFFFFFF if none]

pub fn serialize_chunk(chunk: &Chunk, path: &str) -> Result<(), String> {
    let mut buf: Vec<u8> = Vec::new();

    buf.extend_from_slice(MBC_MAGIC);
    buf.push(MBC_VERSION);

    // Constants
    write_u32(&mut buf, chunk.constants.len() as u32);
    for c in &chunk.constants {
        match c {
            Constant::Int(n) => {
                buf.push(0);
                write_i64(&mut buf, *n);
            }
            Constant::Float(f) => {
                buf.push(1);
                write_f64(&mut buf, *f);
            }
            Constant::Str(s) => {
                buf.push(2);
                write_string(&mut buf, s);
            }
            Constant::Bool(b) => {
                buf.push(3);
                buf.push(if *b { 1 } else { 0 });
            }
        }
    }

    // Names
    write_u32(&mut buf, chunk.names.len() as u32);
    for name in &chunk.names {
        write_string(&mut buf, name);
    }

    // Functions
    write_u32(&mut buf, chunk.functions.len() as u32);
    for func in &chunk.functions {
        write_string(&mut buf, &func.name);
        write_u32(&mut buf, func.start_ip as u32);
        write_u16(&mut buf, func.param_count);
    }

    // Instructions
    write_u32(&mut buf, chunk.instructions.len() as u32);
    for inst in &chunk.instructions {
        buf.push(inst.op as u8);
        write_u16(&mut buf, inst.a);
        write_u16(&mut buf, inst.b);
        write_u16(&mut buf, inst.c);
    }

    // Entry function
    match chunk.entry_func {
        Some(idx) => write_u32(&mut buf, idx as u32),
        None => write_u32(&mut buf, 0xFFFFFFFF),
    }

    std::fs::write(path, &buf).map_err(|e| format!("failed to write {}: {}", path, e))
}

pub fn deserialize_chunk(path: &str) -> Result<Chunk, String> {
    let data = std::fs::read(path).map_err(|e| format!("failed to read {}: {}", path, e))?;
    let mut pos = 0;

    // Magic
    if data.len() < 5 || &data[0..4] != MBC_MAGIC {
        return Err("invalid .mbc file: bad magic".to_string());
    }
    pos += 4;

    let version = data[pos];
    pos += 1;
    if version != MBC_VERSION {
        return Err(format!("unsupported .mbc version: {}", version));
    }

    // Constants
    let num_constants = read_u32(&data, &mut pos)?;
    let mut constants = Vec::new();
    for _ in 0..num_constants {
        let tag = data.get(pos).copied().ok_or("unexpected EOF")?;
        pos += 1;
        let c = match tag {
            0 => Constant::Int(read_i64(&data, &mut pos)?),
            1 => Constant::Float(read_f64(&data, &mut pos)?),
            2 => Constant::Str(read_string(&data, &mut pos)?),
            3 => {
                let b = data.get(pos).copied().ok_or("unexpected EOF")?;
                pos += 1;
                Constant::Bool(b != 0)
            }
            _ => return Err(format!("unknown constant tag: {}", tag)),
        };
        constants.push(c);
    }

    // Names
    let num_names = read_u32(&data, &mut pos)?;
    let mut names = Vec::new();
    for _ in 0..num_names {
        names.push(read_string(&data, &mut pos)?);
    }

    // Functions
    let num_functions = read_u32(&data, &mut pos)?;
    let mut functions = Vec::new();
    for _ in 0..num_functions {
        let name = read_string(&data, &mut pos)?;
        let start_ip = read_u32(&data, &mut pos)? as usize;
        let param_count = read_u16(&data, &mut pos)?;
        functions.push(CompiledFunc { name, start_ip, param_count });
    }

    // Instructions
    let num_instructions = read_u32(&data, &mut pos)?;
    let mut instructions = Vec::new();
    for _ in 0..num_instructions {
        let op_byte = data.get(pos).copied().ok_or("unexpected EOF")?;
        pos += 1;
        let op = OpCode::from_u8(op_byte).ok_or(format!("unknown opcode: {}", op_byte))?;
        let a = read_u16(&data, &mut pos)?;
        let b = read_u16(&data, &mut pos)?;
        let c = read_u16(&data, &mut pos)?;
        instructions.push(Instruction::new(op, a, b, c));
    }

    // Entry function
    let entry_raw = read_u32(&data, &mut pos)?;
    let entry_func = if entry_raw == 0xFFFFFFFF { None } else { Some(entry_raw as usize) };

    Ok(Chunk {
        instructions,
        constants,
        functions,
        entry_func,
        names,
    })
}

// --- Serialization helpers ---

fn write_u16(buf: &mut Vec<u8>, v: u16) {
    buf.extend_from_slice(&v.to_le_bytes());
}

fn write_u32(buf: &mut Vec<u8>, v: u32) {
    buf.extend_from_slice(&v.to_le_bytes());
}

fn write_i64(buf: &mut Vec<u8>, v: i64) {
    buf.extend_from_slice(&v.to_le_bytes());
}

fn write_f64(buf: &mut Vec<u8>, v: f64) {
    buf.extend_from_slice(&v.to_le_bytes());
}

fn write_string(buf: &mut Vec<u8>, s: &str) {
    write_u32(buf, s.len() as u32);
    buf.extend_from_slice(s.as_bytes());
}

fn read_u16(data: &[u8], pos: &mut usize) -> Result<u16, String> {
    if *pos + 2 > data.len() { return Err("unexpected EOF".to_string()); }
    let v = u16::from_le_bytes([data[*pos], data[*pos + 1]]);
    *pos += 2;
    Ok(v)
}

fn read_u32(data: &[u8], pos: &mut usize) -> Result<u32, String> {
    if *pos + 4 > data.len() { return Err("unexpected EOF".to_string()); }
    let v = u32::from_le_bytes([data[*pos], data[*pos + 1], data[*pos + 2], data[*pos + 3]]);
    *pos += 4;
    Ok(v)
}

fn read_i64(data: &[u8], pos: &mut usize) -> Result<i64, String> {
    if *pos + 8 > data.len() { return Err("unexpected EOF".to_string()); }
    let mut bytes = [0u8; 8];
    bytes.copy_from_slice(&data[*pos..*pos + 8]);
    *pos += 8;
    Ok(i64::from_le_bytes(bytes))
}

fn read_f64(data: &[u8], pos: &mut usize) -> Result<f64, String> {
    if *pos + 8 > data.len() { return Err("unexpected EOF".to_string()); }
    let mut bytes = [0u8; 8];
    bytes.copy_from_slice(&data[*pos..*pos + 8]);
    *pos += 8;
    Ok(f64::from_le_bytes(bytes))
}

fn read_string(data: &[u8], pos: &mut usize) -> Result<String, String> {
    let len = read_u32(data, pos)? as usize;
    if *pos + len > data.len() { return Err("unexpected EOF".to_string()); }
    let s = String::from_utf8(data[*pos..*pos + len].to_vec())
        .map_err(|_| "invalid UTF-8 in .mbc".to_string())?;
    *pos += len;
    Ok(s)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lexer::Lexer;
    use crate::parser;
    use crate::sema::SemanticAnalyzer;

    fn compile_source(src: &str) -> Result<Chunk, String> {
        let mut lexer = Lexer::new(src, "test.moli");
        let tokens = lexer.tokenize().map_err(|e| format!("{:?}", e))?;
        let program = parser::parse(tokens, src).map_err(|e| format!("{:?}", e))?;
        let mut sema = SemanticAnalyzer::new();
        sema.analyze(&program).map_err(|e| format!("{:?}", e))?;
        let mut compiler = Compiler::new();
        compiler.compile(&program)
    }

    #[test]
    fn test_compile_hello_world() {
        let src = r#"import stdio

pub mod Example {
    pub container Printing {
        func run() {
            print("Hello, World!")
        }
    }
}

start Example"#;
        let chunk = compile_source(src).expect("should compile");
        assert!(!chunk.instructions.is_empty());
        assert!(!chunk.constants.is_empty());
        assert!(chunk.constants.contains(&Constant::Str("Hello, World!".to_string())));
    }

    #[test]
    fn test_compile_arithmetic() {
        let src = r#"pub mod Main {
    pub container App {
        func run() {
            let x = 1 + 2
            print(x)
        }
    }
}
start Main"#;
        let chunk = compile_source(src).expect("should compile");
        assert!(chunk.instructions.iter().any(|i| i.op == OpCode::Add));
        assert!(chunk.instructions.iter().any(|i| i.op == OpCode::Print));
    }

    #[test]
    fn test_serialize_deserialize() {
        let src = r#"pub mod Main {
    pub container App {
        func run() {
            print("test")
        }
    }
}
start Main"#;
        let chunk = compile_source(src).expect("should compile");

        let tmp_path = "/tmp/moli_test_serialize.mbc";
        serialize_chunk(&chunk, tmp_path).expect("should serialize");
        let loaded = deserialize_chunk(tmp_path).expect("should deserialize");

        assert_eq!(chunk.constants.len(), loaded.constants.len());
        assert_eq!(chunk.instructions.len(), loaded.instructions.len());
        assert_eq!(chunk.names.len(), loaded.names.len());
        assert_eq!(chunk.functions.len(), loaded.functions.len());

        // Cleanup
        let _ = std::fs::remove_file(tmp_path);
    }
}
