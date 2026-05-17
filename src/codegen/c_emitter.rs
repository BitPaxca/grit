use crate::ast::*;

/// Emits C code from a Grit AST — bootstrap backend for running programs
/// before the full LLVM pipeline is ready.
pub struct CEmitter {
    output: String,
    indent: usize,
}

impl CEmitter {
    pub fn new() -> Self {
        Self { output: String::new(), indent: 0 }
    }

    pub fn emit_program(&mut self, program: &SourceFile) -> String {
        self.line("#include <stdio.h>");
        self.line("#include <stdlib.h>");
        self.line("#include <stdint.h>");
        self.line("#include <stdbool.h>");
        self.line("#include <string.h>");
        self.line("");

        // Forward declarations
        for item in &program.items {
            if let Item::Function(f) = item {
                self.emit_fn_forward(f);
            }
        }
        self.line("");

        // Built-in: print function
        self.line("void grit_print(const char* s) { printf(\"%s\\n\", s); }");
        self.line("");

        // Struct definitions
        for item in &program.items {
            if let Item::Struct(s) = item {
                self.emit_struct(s);
            }
        }

        // Function definitions
        for item in &program.items {
            if let Item::Function(f) = item {
                self.emit_function(f);
            }
        }

        self.output.clone()
    }

    fn line(&mut self, s: &str) {
        for _ in 0..self.indent { self.output.push_str("    "); }
        self.output.push_str(s);
        self.output.push('\n');
    }

    fn write(&mut self, s: &str) {
        self.output.push_str(s);
    }

    fn emit_fn_forward(&mut self, f: &FnDecl) {
        let ret = if f.name == "main" { "int".to_string() } else { self.type_to_c(f.return_type.as_deref()) };
        let params = self.params_to_c(&f.params);
        let name = self.mangle_name(&f.name);
        self.line(&format!("{} {}({});", ret, name, params));
    }

    fn emit_struct(&mut self, s: &StructDecl) {
        self.line(&format!("typedef struct {{"));
        self.indent += 1;
        for field in &s.fields {
            let ty = self.type_expr_to_c(&field.ty);
            self.line(&format!("{} {};", ty, field.name));
        }
        self.indent -= 1;
        self.line(&format!("}} {};", s.name));
        self.line("");
    }

    fn emit_function(&mut self, f: &FnDecl) {
        let is_main = f.name == "main";
        let ret = if is_main { "int".to_string() } else { self.type_to_c(f.return_type.as_deref()) };
        let params = self.params_to_c(&f.params);
        let name = self.mangle_name(&f.name);
        self.line(&format!("{} {}({}) {{", ret, name, params));
        self.indent += 1;

        if let Some(body) = &f.body {
            self.emit_block(body, f.return_type.is_some());
        }

        if is_main {
            self.line("return 0;");
        }

        self.indent -= 1;
        self.line("}");
        self.line("");
    }

    fn emit_block(&mut self, block: &Block, _has_return: bool) {
        for stmt in &block.stmts {
            self.emit_stmt(stmt);
        }
        if let Some(expr) = &block.trailing_expr {
            self.write_indent();
            self.write("return ");
            self.emit_expr(expr);
            self.write(";\n");
        }
    }

    fn emit_stmt(&mut self, stmt: &Stmt) {
        match stmt {
            Stmt::Let(l) => {
                self.write_indent();
                if let Some(ty) = &l.ty {
                    self.write(&self.type_expr_to_c(ty));
                } else {
                    self.write(&self.infer_c_type(&l.value));
                }
                if let Pattern::Ident(name, _) = &l.pattern {
                    self.write(&format!(" {} = ", name));
                }
                self.emit_expr(&l.value);
                self.write(";\n");
            }
            Stmt::Var(v) => {
                self.write_indent();
                if let Some(ty) = &v.ty {
                    self.write(&self.type_expr_to_c(ty));
                } else if let Some(val) = &v.value {
                    self.write(&self.infer_c_type(val));
                } else {
                    self.write("int");
                }
                self.write(&format!(" {}", v.name));
                if let Some(val) = &v.value {
                    self.write(" = ");
                    self.emit_expr(val);
                }
                self.write(";\n");
            }
            Stmt::Assign(a) => {
                self.write_indent();
                self.emit_expr(&a.target);
                let op = match a.op {
                    AssignOp::Eq => "=", AssignOp::PlusEq => "+=",
                    AssignOp::MinusEq => "-=", AssignOp::StarEq => "*=",
                    AssignOp::SlashEq => "/=", AssignOp::PercentEq => "%=",
                    AssignOp::AmpEq => "&=", AssignOp::PipeEq => "|=",
                    AssignOp::CaretEq => "^=", AssignOp::ShlEq => "<<=",
                    AssignOp::ShrEq => ">>=",
                };
                self.write(&format!(" {} ", op));
                self.emit_expr(&a.value);
                self.write(";\n");
            }
            Stmt::Expr(e) => {
                self.write_indent();
                self.emit_expr(&e.expr);
                self.write(";\n");
            }
            Stmt::Return(r) => {
                self.write_indent();
                if let Some(val) = &r.value {
                    self.write("return ");
                    self.emit_expr(val);
                    self.write(";\n");
                } else {
                    self.write("return;\n");
                }
            }
            Stmt::Break(_) => { self.line("break;"); }
            Stmt::Continue(_) => { self.line("continue;"); }
            Stmt::Defer(_) => { /* deferred to later */ }
        }
    }

    fn emit_expr(&mut self, expr: &Expr) {
        match expr {
            Expr::IntLiteral(n, _) => self.write(&n.to_string()),
            Expr::FloatLiteral(f, _) => self.write(&format!("{:.6}", f)),
            Expr::StringLiteral(s, _) => self.write(&format!("\"{}\"", s.replace('\\', "\\\\").replace('"', "\\\"").replace('\n', "\\n"))),
            Expr::BoolLiteral(b, _) => self.write(if *b { "true" } else { "false" }),
            Expr::CharLiteral(c, _) => self.write(&format!("'{}'", c)),
            Expr::Ident(name, _) => {
                if name == "print" { self.write("grit_print"); }
                else { self.write(name); }
            }
            Expr::SelfValue(_) => self.write("self"),
            Expr::Binary { left, op, right, .. } => {
                self.write("(");
                self.emit_expr(left);
                let op_str = match op {
                    BinOp::Add => "+", BinOp::Sub => "-", BinOp::Mul => "*",
                    BinOp::Div => "/", BinOp::Mod => "%",
                    BinOp::Eq => "==", BinOp::NotEq => "!=",
                    BinOp::Less => "<", BinOp::Greater => ">",
                    BinOp::LessEq => "<=", BinOp::GreaterEq => ">=",
                    BinOp::And => "&&", BinOp::Or => "||",
                    BinOp::BitAnd => "&", BinOp::BitOr => "|",
                    BinOp::BitXor => "^", BinOp::Shl => "<<", BinOp::Shr => ">>",
                };
                self.write(&format!(" {} ", op_str));
                self.emit_expr(right);
                self.write(")");
            }
            Expr::Unary { op, operand, .. } => {
                let op_str = match op {
                    UnaryOp::Neg => "-", UnaryOp::Not => "!",
                    UnaryOp::BitNot => "~", UnaryOp::Ref => "&",
                    UnaryOp::RefMut => "&", UnaryOp::Deref => "*",
                };
                self.write(op_str);
                self.emit_expr(operand);
            }
            Expr::Call { callee, args, .. } => {
                self.emit_expr(callee);
                self.write("(");
                for (i, arg) in args.iter().enumerate() {
                    if i > 0 { self.write(", "); }
                    self.emit_expr(&arg.value);
                }
                self.write(")");
            }
            Expr::MethodCall { receiver, method, args, .. } => {
                self.emit_expr(receiver);
                self.write(&format!(".{}(", method));
                for (i, arg) in args.iter().enumerate() {
                    if i > 0 { self.write(", "); }
                    self.emit_expr(&arg.value);
                }
                self.write(")");
            }
            Expr::FieldAccess { object, field, .. } => {
                self.emit_expr(object);
                self.write(&format!(".{}", field));
            }
            Expr::Index { object, index, .. } => {
                self.emit_expr(object);
                self.write("[");
                self.emit_expr(index);
                self.write("]");
            }
            Expr::If { condition, then_block, else_block, .. } => {
                self.write("if (");
                self.emit_expr(condition);
                self.write(") {\n");
                self.indent += 1;
                self.emit_block(then_block, false);
                self.indent -= 1;
                self.write_indent();
                self.write("}");
                if let Some(eb) = else_block {
                    self.write(" else {\n");
                    self.indent += 1;
                    self.emit_block(eb, false);
                    self.indent -= 1;
                    self.write_indent();
                    self.write("}");
                }
            }
            Expr::For { pattern, iterator, body, .. } => {
                // Simplified: for i in 0..n → for(int i=0; i<n; i++)
                if let Pattern::Ident(name, _) = pattern.as_ref() {
                    if let Expr::Range { start, end, .. } = iterator.as_ref() {
                        self.write(&format!("for (int {} = ", name));
                        if let Some(s) = start { self.emit_expr(s); } else { self.write("0"); }
                        self.write(&format!("; {} < ", name));
                        if let Some(e) = end { self.emit_expr(e); } else { self.write("0"); }
                        self.write(&format!("; {}++) {{\n", name));
                        self.indent += 1;
                        self.emit_block(body, false);
                        self.indent -= 1;
                        self.write_indent();
                        self.write("}");
                        return;
                    }
                }
                self.write("/* for loop */");
            }
            Expr::While { condition, body, .. } => {
                self.write("while (");
                self.emit_expr(condition);
                self.write(") {\n");
                self.indent += 1;
                self.emit_block(body, false);
                self.indent -= 1;
                self.write_indent();
                self.write("}");
            }
            Expr::Loop { body, .. } => {
                self.write("while (1) {\n");
                self.indent += 1;
                self.emit_block(body, false);
                self.indent -= 1;
                self.write_indent();
                self.write("}");
            }
            Expr::Block(block) => {
                self.write("{\n");
                self.indent += 1;
                self.emit_block(block, false);
                self.indent -= 1;
                self.write_indent();
                self.write("}");
            }
            Expr::Array(items, _) => {
                self.write("{");
                for (i, item) in items.iter().enumerate() {
                    if i > 0 { self.write(", "); }
                    self.emit_expr(item);
                }
                self.write("}");
            }
            Expr::Tuple(items, _) => {
                // C doesn't have tuples — emit as struct literal if needed
                self.write("/* tuple */ 0");
                let _ = items;
            }
            _ => { self.write("/* unsupported expr */"); }
        }
    }

    // ── Helpers ──────────────────────────────────────────

    fn write_indent(&mut self) {
        for _ in 0..self.indent { self.output.push_str("    "); }
    }

    fn mangle_name(&self, name: &str) -> String {
        if name == "main" { "main".to_string() }
        else { format!("grit_{}", name) }
    }

    fn type_to_c(&self, ty: Option<&TypeExpr>) -> String {
        match ty {
            None => "void".to_string(),
            Some(t) => self.type_expr_to_c(t),
        }
    }

    fn type_expr_to_c(&self, ty: &TypeExpr) -> String {
        match ty {
            TypeExpr::Path(path, _) => {
                let name = path.join("_");
                match name.as_str() {
                    "i8" => "int8_t", "i16" => "int16_t", "i32" => "int32_t",
                    "i64" => "int64_t", "i128" => "__int128",
                    "u8" => "uint8_t", "u16" => "uint16_t", "u32" => "uint32_t",
                    "u64" => "uint64_t", "u128" => "unsigned __int128",
                    "usize" => "size_t", "isize" => "ptrdiff_t",
                    "f32" => "float", "f64" => "double",
                    "bool" => "bool", "char" => "char",
                    "String" => "const char*",
                    _ => return name,
                }.to_string()
            }
            TypeExpr::Reference { inner, .. } => format!("{}*", self.type_expr_to_c(inner)),
            TypeExpr::Pointer { inner, .. } => format!("{}*", self.type_expr_to_c(inner)),
            TypeExpr::Array { element, .. } => format!("{}[]", self.type_expr_to_c(element)),
            _ => "void".to_string(),
        }
    }

    fn params_to_c(&self, params: &[Param]) -> String {
        if params.is_empty() { return "void".to_string(); }
        params.iter().map(|p| {
            let ty = p.ty.as_ref().map(|t| self.type_expr_to_c(t)).unwrap_or("int".to_string());
            format!("{} {}", ty, p.name)
        }).collect::<Vec<_>>().join(", ")
    }

    fn infer_c_type(&self, expr: &Expr) -> String {
        match expr {
            Expr::IntLiteral(_, _) => "int".to_string(),
            Expr::FloatLiteral(_, _) => "double".to_string(),
            Expr::StringLiteral(_, _) => "const char*".to_string(),
            Expr::BoolLiteral(_, _) => "bool".to_string(),
            Expr::CharLiteral(_, _) => "char".to_string(),
            Expr::Binary { op, .. } => {
                match op {
                    BinOp::Eq | BinOp::NotEq | BinOp::Less | BinOp::Greater
                    | BinOp::LessEq | BinOp::GreaterEq | BinOp::And | BinOp::Or => "bool".to_string(),
                    _ => "int".to_string(),
                }
            }
            _ => "int".to_string(),
        }
    }
}
