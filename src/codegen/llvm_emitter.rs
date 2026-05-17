use crate::ast::*;

/// Emits LLVM IR text from a Grit AST — the proper codegen backend.
/// Output is a .ll file that can be compiled with `clang` or `llc`.
pub struct LLVMEmitter {
    output: String,
    reg: u32,       // SSA register counter
    str_id: u32,    // string constant counter
    strings: Vec<(String, usize)>, // (value, byte_len including null)
}

impl LLVMEmitter {
    pub fn new() -> Self {
        Self { output: String::new(), reg: 0, str_id: 0, strings: Vec::new() }
    }

    fn next_reg(&mut self) -> u32 {
        let r = self.reg;
        self.reg += 1;
        r
    }

    fn line(&mut self, s: &str) {
        self.output.push_str(s);
        self.output.push('\n');
    }

    pub fn emit_program(&mut self, program: &SourceFile) -> String {
        // Collect string constants first
        for item in &program.items {
            if let Item::Function(f) = item {
                if let Some(body) = &f.body {
                    self.collect_strings_block(body);
                }
            }
        }

        // Header
        self.line("; Grit LLVM IR output");
        self.line("target triple = \"x86_64-pc-windows-msvc\"");
        self.line("");

        // Declare printf
        self.line("declare i32 @printf(i8*, ...)");
        self.line("");

        // String constants
        for (id, (s, len)) in self.strings.clone().iter().enumerate() {
            let escaped = s.replace('\\', "\\5C").replace('"', "\\22");
            self.line(&format!(
                "@.str.{} = private unnamed_addr constant [{} x i8] c\"{}\\0A\\00\"",
                id, len, escaped
            ));
        }
        if !self.strings.is_empty() { self.line(""); }

        // Functions
        for item in &program.items {
            if let Item::Function(f) = item {
                self.emit_function(f);
            }
        }

        self.output.clone()
    }

    fn collect_strings_block(&mut self, block: &Block) {
        for stmt in &block.stmts {
            self.collect_strings_stmt(stmt);
        }
        if let Some(expr) = &block.trailing_expr {
            self.collect_strings_expr(expr);
        }
    }

    fn collect_strings_stmt(&mut self, stmt: &Stmt) {
        match stmt {
            Stmt::Let(l) => self.collect_strings_expr(&l.value),
            Stmt::Var(v) => { if let Some(val) = &v.value { self.collect_strings_expr(val); } }
            Stmt::Expr(e) => self.collect_strings_expr(&e.expr),
            Stmt::Assign(a) => { self.collect_strings_expr(&a.target); self.collect_strings_expr(&a.value); }
            Stmt::Return(r) => { if let Some(v) = &r.value { self.collect_strings_expr(v); } }
            _ => {}
        }
    }

    fn collect_strings_expr(&mut self, expr: &Expr) {
        match expr {
            Expr::StringLiteral(s, _) => {
                let len = s.len() + 2; // +\n +\0
                self.strings.push((s.clone(), len));
            }
            Expr::Call { args, callee, .. } => {
                self.collect_strings_expr(callee);
                for a in args { self.collect_strings_expr(&a.value); }
            }
            Expr::Binary { left, right, .. } => {
                self.collect_strings_expr(left);
                self.collect_strings_expr(right);
            }
            Expr::If { condition, then_block, else_block, .. } => {
                self.collect_strings_expr(condition);
                self.collect_strings_block(then_block);
                if let Some(eb) = else_block { self.collect_strings_block(eb); }
            }
            _ => {}
        }
    }

    fn emit_function(&mut self, f: &FnDecl) {
        let ret_ty = if f.return_type.is_some() { "i32" } else { "i32" }; // main returns i32
        let name = if f.name == "main" { "main" } else { &f.name };

        self.reg = 0;
        self.line(&format!("define {} @{}() {{", ret_ty, name));

        if let Some(body) = &f.body {
            self.emit_block_ir(body);
        }

        self.line("  ret i32 0");
        self.line("}");
        self.line("");
    }

    fn emit_block_ir(&mut self, block: &Block) {
        for stmt in &block.stmts {
            self.emit_stmt_ir(stmt);
        }
    }

    fn emit_stmt_ir(&mut self, stmt: &Stmt) {
        match stmt {
            Stmt::Expr(e) => { self.emit_expr_ir(&e.expr); }
            Stmt::Let(l) => { self.emit_expr_ir(&l.value); }
            _ => {}
        }
    }

    fn emit_expr_ir(&mut self, expr: &Expr) -> Option<u32> {
        match expr {
            Expr::Call { callee, args, .. } => {
                if let Expr::Ident(name, _) = callee.as_ref() {
                    if name == "print" {
                        if let Some(Expr::StringLiteral(s, _)) = args.first().map(|a| &a.value) {
                            // Find the string constant ID
                            let str_idx = self.strings.iter().position(|(v, _)| v == s).unwrap_or(0);
                            let len = self.strings[str_idx].1;
                            let r = self.next_reg();
                            self.line(&format!(
                                "  %{} = call i32 (i8*, ...) @printf(i8* getelementptr ([{} x i8], [{} x i8]* @.str.{}, i32 0, i32 0))",
                                r, len, len, str_idx
                            ));
                            return Some(r);
                        }
                    }
                }
                None
            }
            Expr::IntLiteral(n, _) => {
                let r = self.next_reg();
                self.line(&format!("  %{} = add i32 0, {}", r, n));
                Some(r)
            }
            _ => None,
        }
    }
}
