use crate::ast::*;
use std::collections::HashMap;

/// Emits C code from a Grit AST — bootstrap backend for running programs
/// before the full LLVM pipeline is ready.
pub struct CEmitter {
    output: String,
    indent: usize,
    fn_return_types: HashMap<String, String>,
    enums: HashMap<String, EnumDecl>,
}

impl CEmitter {
    pub fn new() -> Self {
        Self { output: String::new(), indent: 0, fn_return_types: HashMap::new(), enums: HashMap::new() }
    }

    pub fn emit_program(&mut self, program: &SourceFile) -> String {
        // Pre-collect function return types for type inference
        for item in &program.items {
            if let Item::Function(f) = item {
                let ret = if f.name == "main" {
                    "int".to_string()
                } else {
                    self.type_to_c(f.return_type.as_deref())
                };
                self.fn_return_types.insert(f.name.clone(), ret);
            } else if let Item::Enum(e) = item {
                self.enums.insert(e.name.clone(), e.clone());
            }
        }

        self.line("#include <stdio.h>");
        self.line("#include <stdlib.h>");
        self.line("#include <stdint.h>");
        self.line("#include <stdbool.h>");
        self.line("#include <string.h>");
        self.line("");
        // Forward-declare opaque types used by the stdlib
        self.line("typedef struct { const char** kinds; const char** lexemes; int len; int cap; } TokenList;");
        self.line("typedef struct { const char** names; int* type_ids; int* depths; int len; int cap; } SymbolTable;");
        self.line("typedef struct { int* data; int len; int cap; } GritVec;");
        self.line("typedef struct { const char** items; int len; int cap; } StringArray;");
        self.line("");

        // Struct and Enum definitions
        for item in &program.items {
            if let Item::Struct(s) = item {
                self.emit_struct(s);
            } else if let Item::Enum(e) = item {
                self.emit_enum(e);
            }
        }

        // Forward declarations
        for item in &program.items {
            if let Item::Function(f) = item {
                self.emit_fn_forward(f);
            }
        }
        self.line("");

        // ── Grit Standard Library ──
        self.line("#include <math.h>");
        self.line("");
        self.line("// I/O");
        self.line(r#"void grit_print(const char* s) { printf("%s\n", s); }"#);
        self.line(r#"void grit_print_inline(const char* s) { printf("%s", s); }"#);
        self.line(r#"void grit_println(const char* s) { printf("%s\n", s); }"#);
        self.line(r#"void grit_eprint(const char* s) { fprintf(stderr, "%s", s); }"#);
        self.line(r#"void grit_eprintln(const char* s) { fprintf(stderr, "%s\n", s); }"#);
        self.line(r#"const char* grit_readln() { static char buf[4096]; if (fgets(buf, sizeof(buf), stdin)) { buf[strcspn(buf, "\n")] = 0; return buf; } return ""; }"#);
        self.line("");
        self.line("// Assertions & Debugging");
        self.line(r#"void grit_assert(bool cond) { if (!cond) { fprintf(stderr, "assertion failed\n"); exit(1); } }"#);
        self.line(r#"void grit_assert_eq(int left, int right) { if (left != right) { fprintf(stderr, "assertion failed: %d != %d\n", left, right); exit(1); } }"#);
        self.line(r#"void grit_panic(const char* msg) { fprintf(stderr, "panic: %s\n", msg); exit(1); }"#);
        self.line(r#"void grit_unreachable() { fprintf(stderr, "unreachable code reached\n"); exit(1); }"#);
        self.line(r#"void grit_todo() { fprintf(stderr, "not yet implemented\n"); exit(1); }"#);
        self.line("");
        self.line("// Type conversions");
        self.line(r#"const char* grit_to_string(int value) { static char buf[32]; snprintf(buf, sizeof(buf), "%d", value); return buf; }"#);
        self.line("int grit_to_int(const char* s) { return atoi(s); }");
        self.line("double grit_to_float(const char* s) { return atof(s); }");
        self.line("");
        self.line("// Math");
        self.line("int grit_abs(int x) { return x < 0 ? -x : x; }");
        self.line("int grit_min(int a, int b) { return a < b ? a : b; }");
        self.line("int grit_max(int a, int b) { return a > b ? a : b; }");
        self.line("int grit_clamp(int x, int lo, int hi) { return x < lo ? lo : (x > hi ? hi : x); }");
        self.line("double grit_sqrt(double x) { return sqrt(x); }");
        self.line("double grit_pow(double base, double exp) { return pow(base, exp); }");
        self.line("");
        self.line("// Process control");
        self.line("void grit_exit(int code) { exit(code); }");
        self.line("");
        self.line("// String operations");
        self.line("int grit_len(const char* s) { return (int)strlen(s); }");
        self.line("char grit_char_at(const char* s, int index) { if (index < 0 || (size_t)index >= strlen(s)) { fprintf(stderr, \"panic: index out of bounds\\n\"); exit(1); } return s[index]; }");
        self.line(r#"const char* grit_substring(const char* s, int start, int end) { size_t len = strlen(s); if (start < 0 || end < 0 || (size_t)start > len || (size_t)end > len || start > end) { fprintf(stderr, "panic: invalid substring indices\n"); exit(1); } size_t sub_len = (size_t)(end - start); char* buf = malloc(sub_len + 1); strncpy(buf, s + start, sub_len); buf[sub_len] = '\0'; return buf; }"#);
        self.line("bool grit_string_eq(const char* s1, const char* s2) { return strcmp(s1, s2) == 0; }");
        self.line("");
        self.line("// File I/O");
        self.line(r#"const char* grit_read_file(const char* path) { FILE* f = fopen(path, "rb"); if (!f) { fprintf(stderr, "panic: could not open file %s\n", path); exit(1); } fseek(f, 0, SEEK_END); long fsize = ftell(f); fseek(f, 0, SEEK_SET); char* string = malloc(fsize + 1); fread(string, fsize, 1, f); fclose(f); string[fsize] = 0; return string; }"#);
        self.line("");
        self.line("// Integer printing");
        self.line(r#"void grit_print_int(int x) { printf("%d\n", x); }"#);
        self.line(r#"void grit_print_int_inline(int x) { printf("%d", x); }"#);
        self.line("");
        self.line("// TokenList — growable array of (kind, lexeme) pairs for the self-hosted lexer");
        self.line("TokenList* grit_token_list_new() { TokenList* tl = malloc(sizeof(TokenList)); tl->len = 0; tl->cap = 256; tl->kinds = malloc(sizeof(const char*) * 256); tl->lexemes = malloc(sizeof(const char*) * 256); return tl; }");
        self.line("void grit_token_list_push(TokenList* tl, const char* kind, const char* lexeme) { if (tl->len >= tl->cap) { tl->cap *= 2; tl->kinds = realloc(tl->kinds, sizeof(const char*) * tl->cap); tl->lexemes = realloc(tl->lexemes, sizeof(const char*) * tl->cap); } tl->kinds[tl->len] = kind; tl->lexemes[tl->len] = lexeme; tl->len++; }");
        self.line("const char* grit_token_list_get_kind(TokenList* tl, int index) { if (index < 0 || index >= tl->len) { fprintf(stderr, \"panic: token index out of bounds\\n\"); exit(1); } return tl->kinds[index]; }");
        self.line("const char* grit_token_list_get_lexeme(TokenList* tl, int index) { if (index < 0 || index >= tl->len) { fprintf(stderr, \"panic: token index out of bounds\\n\"); exit(1); } return tl->lexemes[index]; }");
        self.line("int grit_token_list_len(TokenList* tl) { return tl->len; }");
        self.line("");
        self.line("// SymbolTable — growable array of (name, type_id, depth) for the self-hosted typechecker");
        self.line("SymbolTable* grit_symtab_new() { SymbolTable* st = malloc(sizeof(SymbolTable)); st->len = 0; st->cap = 256; st->names = malloc(sizeof(const char*) * 256); st->type_ids = malloc(sizeof(int) * 256); st->depths = malloc(sizeof(int) * 256); return st; }");
        self.line("void grit_symtab_push(SymbolTable* st, const char* name, int type_id, int depth) { if (st->len >= st->cap) { st->cap *= 2; st->names = realloc(st->names, sizeof(const char*) * st->cap); st->type_ids = realloc(st->type_ids, sizeof(int) * st->cap); st->depths = realloc(st->depths, sizeof(int) * st->cap); } st->names[st->len] = name; st->type_ids[st->len] = type_id; st->depths[st->len] = depth; st->len++; }");
        self.line("void grit_symtab_pop(SymbolTable* st) { if (st->len > 0) st->len--; }");
        self.line("int grit_symtab_len(SymbolTable* st) { return st->len; }");
        self.line("const char* grit_symtab_get_name(SymbolTable* st, int index) { if (index < 0 || index >= st->len) { fprintf(stderr, \"panic: symtab index out of bounds\\n\"); exit(1); } return st->names[index]; }");
        self.line("int grit_symtab_get_type(SymbolTable* st, int index) { if (index < 0 || index >= st->len) { fprintf(stderr, \"panic: symtab index out of bounds\\n\"); exit(1); } return st->type_ids[index]; }");
        self.line("int grit_symtab_get_depth(SymbolTable* st, int index) { if (index < 0 || index >= st->len) { fprintf(stderr, \"panic: symtab index out of bounds\\n\"); exit(1); } return st->depths[index]; }");
        self.line("");
        self.line("// Dynamic Memory");
        self.line("intptr_t grit_alloc(int size) { return (intptr_t)malloc((size_t)size); }");
        self.line("void grit_free(intptr_t ptr) { free((void*)ptr); }");
        self.line("intptr_t grit_realloc(intptr_t ptr, int new_size) { return (intptr_t)realloc((void*)ptr, (size_t)new_size); }");
        self.line("");
        self.line("// String Builder");
        self.line(r#"const char* grit_string_new() { char* s = malloc(1); s[0] = '\0'; return s; }"#);
        self.line(r#"const char* grit_string_concat(const char* a, const char* b) { size_t la = strlen(a), lb = strlen(b); char* r = malloc(la + lb + 1); memcpy(r, a, la); memcpy(r + la, b, lb); r[la + lb] = '\0'; return r; }"#);
        self.line(r#"const char* grit_string_from_int(int value) { char* buf = malloc(32); snprintf(buf, 32, "%d", value); return buf; }"#);
        self.line(r#"const char* grit_string_push_char(const char* s, char c) { size_t len = strlen(s); char* r = malloc(len + 2); memcpy(r, s, len); r[len] = c; r[len + 1] = '\0'; return r; }"#);
        self.line(r#"bool grit_string_contains(const char* haystack, const char* needle) { return strstr(haystack, needle) != NULL; }"#);
        self.line(r#"bool grit_string_starts_with(const char* s, const char* prefix) { return strncmp(s, prefix, strlen(prefix)) == 0; }"#);
        self.line(r#"bool grit_string_ends_with(const char* s, const char* suffix) { size_t sl = strlen(s), xl = strlen(suffix); if (xl > sl) return false; return strcmp(s + sl - xl, suffix) == 0; }"#);
        self.line(r#"const char* grit_string_replace(const char* s, const char* from, const char* to) { size_t fl = strlen(from), tl = strlen(to), sl = strlen(s); char* result = malloc(sl * 4 + 1); size_t ri = 0; for (size_t i = 0; i < sl; ) { if (i + fl <= sl && strncmp(s + i, from, fl) == 0) { memcpy(result + ri, to, tl); ri += tl; i += fl; } else { result[ri++] = s[i++]; } } result[ri] = '\0'; return result; }"#);
        self.line(r#"const char* grit_string_trim(const char* s) { while (*s == ' ' || *s == '\t' || *s == '\n' || *s == '\r') s++; size_t len = strlen(s); while (len > 0 && (s[len-1] == ' ' || s[len-1] == '\t' || s[len-1] == '\n' || s[len-1] == '\r')) len--; char* r = malloc(len + 1); memcpy(r, s, len); r[len] = '\0'; return r; }"#);
        self.line(r#"#include <ctype.h>"#);
        self.line(r#"const char* grit_string_to_upper(const char* s) { size_t len = strlen(s); char* r = malloc(len + 1); for (size_t i = 0; i < len; i++) r[i] = (char)toupper((unsigned char)s[i]); r[len] = '\0'; return r; }"#);
        self.line(r#"const char* grit_string_to_lower(const char* s) { size_t len = strlen(s); char* r = malloc(len + 1); for (size_t i = 0; i < len; i++) r[i] = (char)tolower((unsigned char)s[i]); r[len] = '\0'; return r; }"#);
        self.line(r#"StringArray* grit_string_split(const char* s, const char* delim) { StringArray* sa = malloc(sizeof(StringArray)); sa->len = 0; sa->cap = 16; sa->items = malloc(sizeof(const char*) * 16); char* copy = malloc(strlen(s) + 1); strcpy(copy, s); size_t dl = strlen(delim); char* p = copy; while (*p) { char* found = strstr(p, delim); if (found) { *found = '\0'; if (sa->len >= sa->cap) { sa->cap *= 2; sa->items = realloc(sa->items, sizeof(const char*) * sa->cap); } sa->items[sa->len++] = p; p = found + dl; } else { if (sa->len >= sa->cap) { sa->cap *= 2; sa->items = realloc(sa->items, sizeof(const char*) * sa->cap); } sa->items[sa->len++] = p; break; } } return sa; }"#);
        self.line("");
        self.line("// Vec (Dynamic Array)");
        self.line("GritVec* grit_vec_new() { GritVec* v = malloc(sizeof(GritVec)); v->len = 0; v->cap = 16; v->data = malloc(sizeof(int) * 16); return v; }");
        self.line("void grit_vec_push(GritVec* v, int value) { if (v->len >= v->cap) { v->cap *= 2; v->data = realloc(v->data, sizeof(int) * v->cap); } v->data[v->len++] = value; }");
        self.line(r#"int grit_vec_pop(GritVec* v) { if (v->len == 0) { fprintf(stderr, "panic: pop from empty vec\n"); exit(1); } return v->data[--v->len]; }"#);
        self.line(r#"int grit_vec_get(GritVec* v, int index) { if (index < 0 || index >= v->len) { fprintf(stderr, "panic: vec index out of bounds\n"); exit(1); } return v->data[index]; }"#);
        self.line(r#"void grit_vec_set(GritVec* v, int index, int value) { if (index < 0 || index >= v->len) { fprintf(stderr, "panic: vec index out of bounds\n"); exit(1); } v->data[index] = value; }"#);
        self.line("int grit_vec_len(GritVec* v) { return v->len; }");
        self.line("void grit_vec_clear(GritVec* v) { v->len = 0; }");
        self.line("");
        self.line("// File I/O (write)");
        self.line(r#"bool grit_write_file(const char* path, const char* contents) { FILE* f = fopen(path, "w"); if (!f) return false; fputs(contents, f); fclose(f); return true; }"#);
        self.line(r#"bool grit_append_file(const char* path, const char* contents) { FILE* f = fopen(path, "a"); if (!f) return false; fputs(contents, f); fclose(f); return true; }"#);
        self.line(r#"#ifdef _WIN32"#);
        self.line(r#"#include <io.h>"#);
        self.line(r#"bool grit_file_exists(const char* path) { return _access(path, 0) == 0; }"#);
        self.line(r#"#else"#);
        self.line(r#"#include <unistd.h>"#);
        self.line(r#"bool grit_file_exists(const char* path) { return access(path, F_OK) == 0; }"#);
        self.line(r#"#endif"#);
        self.line(r#"bool grit_delete_file(const char* path) { return remove(path) == 0; }"#);
        self.line("");

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

    fn emit_enum(&mut self, e: &EnumDecl) {
        // 1. Emit the tag enum
        self.line(&format!("typedef enum {{"));
        self.indent += 1;
        for variant in &e.variants {
            self.line(&format!("{}_{},", e.name, variant.name));
        }
        self.indent -= 1;
        self.line(&format!("}} {}_Tag;", e.name));
        self.line("");

        // 2. Emit the tagged union struct
        self.line(&format!("typedef struct {{"));
        self.indent += 1;
        self.line(&format!("{}_Tag tag;", e.name));
        
        // Only emit union if there are payload variants
        let has_payload = e.variants.iter().any(|v| !matches!(v.fields, VariantKind::Unit));
        if has_payload {
            self.line("union {");
            self.indent += 1;
            for variant in &e.variants {
                match &variant.fields {
                    VariantKind::Unit => {}
                    VariantKind::Tuple(types) => {
                        self.line(&format!("struct {{"));
                        self.indent += 1;
                        for (i, ty) in types.iter().enumerate() {
                            let c_ty = self.type_expr_to_c(ty);
                            self.line(&format!("{} _{};", c_ty, i));
                        }
                        self.indent -= 1;
                        self.line(&format!("}} {};", variant.name));
                    }
                    VariantKind::Struct(fields) => {
                        self.line(&format!("struct {{"));
                        self.indent += 1;
                        for field in fields {
                            let c_ty = self.type_expr_to_c(&field.ty);
                            self.line(&format!("{} {};", c_ty, field.name));
                        }
                        self.indent -= 1;
                        self.line(&format!("}} {};", variant.name));
                    }
                }
            }
            self.indent -= 1;
            self.line("} payload;");
        }

        self.indent -= 1;
        self.line(&format!("}} {};", e.name));
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
            Expr::StringLiteral(s, _) => {
                let escaped = s.replace('\\', "\\\\")
                               .replace('"', "\\\"")
                               .replace('\n', "\\n")
                               .replace('\r', "\\r")
                               .replace('\t', "\\t");
                self.write(&format!("\"{}\"", escaped))
            }
            Expr::BoolLiteral(b, _) => self.write(if *b { "true" } else { "false" }),
            Expr::CharLiteral(c, _) => {
                let escaped = match c {
                    '\n' => "\\n".to_string(),
                    '\r' => "\\r".to_string(),
                    '\t' => "\\t".to_string(),
                    '\\' => "\\\\".to_string(),
                    '\'' => "\\'".to_string(),
                    '\0' => "\\0".to_string(),
                    _ => c.to_string(),
                };
                self.write(&format!("'{}'", escaped))
            }
            Expr::Ident(name, _) => {
                self.write(name);
            }
            Expr::SelfValue(_) => self.write("self"),
            Expr::Path(path, _) => {
                self.write(&path.join("_"));
            }
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
                // Mangle function names so user-defined functions get grit_ prefix
                if let Expr::Ident(name, _) = callee.as_ref() {
                    self.write(&self.mangle_name(name));
                } else if let Expr::Path(path, _) = callee.as_ref() {
                    self.write(&path.join("_"));
                } else {
                    self.emit_expr(callee);
                }
                self.write("(");
                for (i, arg) in args.iter().enumerate() {
                    if i > 0 { self.write(", "); }
                    self.emit_expr(&arg.value);
                }
                self.write(")");
            }
            Expr::MethodCall { receiver, method, args, .. } => {
                // Check if this is an Enum variant constructor: EnumName.Variant(args...)
                if let Expr::Ident(enum_name, _) = receiver.as_ref() {
                    if self.enums.contains_key(enum_name) {
                        self.write(&format!("({}){{ .tag = {}_{}, .payload = {{ .{} = {{ ", enum_name, enum_name, method, method));
                        for (i, arg) in args.iter().enumerate() {
                            if i > 0 { self.write(", "); }
                            self.write(&format!("._{} = ", i));
                            self.emit_expr(&arg.value);
                        }
                        self.write(" } } }");
                        return;
                    }
                }
                
                self.emit_expr(receiver);
                self.write(&format!(".{}(", method));
                for (i, arg) in args.iter().enumerate() {
                    if i > 0 { self.write(", "); }
                    self.emit_expr(&arg.value);
                }
                self.write(")");
            }
            Expr::FieldAccess { object, field, .. } => {
                // Check if this is an Enum Unit variant: EnumName.Variant
                if let Expr::Ident(enum_name, _) = object.as_ref() {
                    if self.enums.contains_key(enum_name) {
                        self.write(&format!("({}){{ .tag = {}_{} }}", enum_name, enum_name, field));
                        return;
                    }
                }
                
                self.emit_expr(object);
                self.write(&format!(".{}", field));
            }
            Expr::Index { object, index, .. } => {
                self.emit_expr(object);
                self.write("[");
                self.emit_expr(index);
                self.write("]");
            }
            Expr::If { condition, then_block, else_ifs, else_block, .. } => {
                self.write("if (");
                self.emit_expr(condition);
                self.write(") {\n");
                self.indent += 1;
                self.emit_block(then_block, false);
                self.indent -= 1;
                self.write_indent();
                self.write("}");
                for (cond, block) in else_ifs {
                    self.write(" else if (");
                    self.emit_expr(cond);
                    self.write(") {\n");
                    self.indent += 1;
                    self.emit_block(block, false);
                    self.indent -= 1;
                    self.write_indent();
                    self.write("}");
                }
                if let Some(eb) = else_block {
                    self.write(" else {\n");
                    self.indent += 1;
                    self.emit_block(eb, false);
                    self.indent -= 1;
                    self.write_indent();
                    self.write("}");
                }
            }
            Expr::Match { subject, arms, .. } => {
                for (i, arm) in arms.iter().enumerate() {
                    if i > 0 {
                        self.write(" else ");
                    }
                    
                    match &arm.pattern {
                        Pattern::Enum { path, variant, fields, .. } => {
                            let enum_name = path.last().unwrap();
                            self.write("if (");
                            self.emit_expr(subject);
                            self.write(&format!(".tag == {}_{}) {{\n", enum_name, variant));
                            self.indent += 1;
                            
                            // Bind fields
                            if !fields.is_empty() {
                                let mut tuple_types = None;
                                if let Some(enum_decl) = self.enums.get(enum_name) {
                                    if let Some(var_decl) = enum_decl.variants.iter().find(|v| &v.name == variant) {
                                        if let VariantKind::Tuple(types) = &var_decl.fields {
                                            tuple_types = Some(types.clone());
                                        }
                                    }
                                }
                                
                                if let Some(types) = tuple_types {
                                    for (j, field_pat) in fields.iter().enumerate() {
                                        if let Pattern::Ident(name, _) = field_pat {
                                            if name != "_" {
                                                let c_ty = self.type_expr_to_c(&types[j]);
                                                self.write_indent();
                                                self.write(&format!("{} {} = ", c_ty, name));
                                                self.emit_expr(subject);
                                                self.write(&format!(".payload.{}._{};\n", variant, j));
                                            }
                                        }
                                    }
                                }
                            }
                            
                            if let Expr::Block(b) = &arm.body {
                                self.emit_block(b, false);
                            } else {
                                self.write_indent();
                                self.emit_expr(&arm.body);
                                self.write(";\n");
                            }
                            
                            self.indent -= 1;
                            self.write_indent();
                            self.write_indent();
                            self.write("}");
                        }
                        Pattern::Literal(lit) => {
                            self.write("if (");
                            self.emit_expr(subject);
                            self.write(" == ");
                            self.emit_expr(lit);
                            self.write(") {\n");
                            self.indent += 1;
                            if let Expr::Block(b) = &arm.body {
                                self.emit_block(b, false);
                            } else {
                                self.write_indent();
                                self.emit_expr(&arm.body);
                                self.write(";\n");
                            }
                            self.indent -= 1;
                            self.write_indent();
                            self.write("}");
                        }
                        Pattern::Wildcard(_) | Pattern::Ident(_, _) => {
                            self.write("{\n");
                            self.indent += 1;
                            if let Expr::Block(b) = &arm.body {
                                self.emit_block(b, false);
                            } else {
                                self.write_indent();
                                self.emit_expr(&arm.body);
                                self.write(";\n");
                            }
                            self.indent -= 1;
                            self.write_indent();
                            self.write("}");
                        }
                        _ => {
                            self.write("/* unsupported match pattern */");
                        }
                    }
                }
                self.write("\n");
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
            Expr::Spawn { kind, body, .. } => {
                let kind_str = match kind {
                    SpawnKind::Task => "task",
                    SpawnKind::Thread => "thread",
                };
                self.write(&format!("/* spawn {} */\n", kind_str));
                self.write_indent();
                self.write("{\n");
                self.indent += 1;
                self.emit_block(body, false);
                self.indent -= 1;
                self.write_indent();
                self.write("}");
            }
            _ => { self.write(&format!("/* unsupported expr: {:?} */", expr)); }
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
                    "TokenList" => return "TokenList*".to_string(),
                    "SymbolTable" => return "SymbolTable*".to_string(),
                    "Vec" => return "GritVec*".to_string(),
                    "StringArray" => return "StringArray*".to_string(),
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
            Expr::Path(path, _) => {
                path.join("_")
            }
            Expr::MethodCall { receiver, .. } => {
                if let Expr::Ident(enum_name, _) = receiver.as_ref() {
                    if self.enums.contains_key(enum_name) {
                        return enum_name.clone();
                    }
                }
                "void".to_string()
            }
            Expr::FieldAccess { object, .. } => {
                if let Expr::Ident(enum_name, _) = object.as_ref() {
                    if self.enums.contains_key(enum_name) {
                        return enum_name.clone();
                    }
                }
                "void".to_string()
            }
            Expr::Call { callee, .. } => {
                if let Expr::Ident(name, _) = callee.as_ref() {
                    match name.as_str() {
                        "read_file" | "substring" | "to_string" | "readln" | "string_new" | "string_concat"
                        | "string_from_int" | "string_push_char" | "string_replace" | "string_trim"
                        | "string_to_upper" | "string_to_lower"
                        | "token_list_get_kind" | "token_list_get_lexeme" | "symtab_get_name" => "const char*".to_string(),
                        "char_at" => "char".to_string(),
                        "string_eq" | "string_contains" | "string_starts_with" | "string_ends_with"
                        | "write_file" | "append_file" | "file_exists" | "delete_file" => "bool".to_string(),
                        "vec_new" => "GritVec*".to_string(),
                        "string_split" => "StringArray*".to_string(),
                        "token_list_new" => "TokenList*".to_string(),
                        "symtab_new" => "SymbolTable*".to_string(),
                        "len" | "token_list_len" | "symtab_len" | "symtab_get_type" | "symtab_get_depth"
                        | "vec_len" | "vec_get" | "vec_pop" | "alloc" | "realloc" => "int".to_string(),
                        _ => {
                            // Look up user-defined function return types
                            if let Some(ret) = self.fn_return_types.get(name.as_str()) {
                                ret.clone()
                            } else {
                                "int".to_string()
                            }
                        }
                    }
                } else {
                    "int".to_string()
                }
            }
            _ => "int".to_string(),
        }
    }
}
