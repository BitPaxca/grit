use crate::ast::*;
use super::types::Ty;
use super::env::TypeEnv;

pub struct TypeChecker {
    env: TypeEnv,
    pub errors: Vec<crate::error::CompilerError>,
    next_infer: u32,
    current_stmt_borrows: Vec<(String, Vec<String>, bool)>, // (var_name, field_path, is_mut)
    pub functions: std::collections::HashMap<String, crate::ast::FnDecl>,
    current_safety: crate::ast::SafetyMode,
}

impl TypeChecker {
    pub fn new() -> Self {
        Self { env: TypeEnv::new(), errors: Vec::new(), next_infer: 0, current_stmt_borrows: Vec::new(), functions: std::collections::HashMap::new(), current_safety: crate::ast::SafetyMode::Safe }
    }

    pub fn has_errors(&self) -> bool { !self.errors.is_empty() }
    
    pub fn error_strings(&self) -> Vec<String> {
        self.errors.iter().map(|e| e.message.clone()).collect()
    }

    fn error(&mut self, span: crate::lexer::Span, msg: &str) {
        self.errors.push(crate::error::CompilerError {
            category: crate::error::ErrorCategory::Type,
            severity: crate::error::Severity::Error,
            span,
            code: "E0308".to_string(), // generic type error code for now
            message: msg.to_string(),
            explanation: None,
            suggestion: None,
            related: Vec::new(),
        });
    }

    fn error_rich(&mut self, err: crate::error::CompilerError) {
        self.errors.push(err);
    }

    fn fresh_infer(&mut self) -> Ty {
        let id = self.next_infer;
        self.next_infer += 1;
        Ty::Infer(id)
    }

    // ── Top-level checking ───────────────────────────────

    pub fn check_program(&mut self, program: &SourceFile) {
        // Register standard library built-in functions
        for builtin in crate::stdlib::builtin_functions() {
            let params: Vec<Ty> = builtin.params.iter().map(|(_, ty)| ty.clone()).collect();
            let ret = builtin.return_type.clone();
            self.env.define_fn(&builtin.name, Ty::Function { params, ret: Box::new(ret) });
        }

        // First pass: register all type and function declarations
        for item in &program.items {
            self.register_item(item);
        }
        // Second pass: check function bodies
        for item in &program.items {
            self.check_item(item);
        }
    }

    fn register_item(&mut self, item: &Item) {
        match item {
            Item::Function(f) => {
                let params: Vec<Ty> = f.params.iter().map(|p| self.resolve_param_type(p)).collect();
                let ret = f.return_type.as_ref().map(|t| self.resolve_type(t)).unwrap_or(Ty::Unit);
                self.env.define_fn(&f.name, Ty::Function { params, ret: Box::new(ret) });
                self.functions.insert(f.name.clone(), f.clone());
            }
            Item::Struct(s) => {
                let fields: Vec<(String, Ty)> = s.fields.iter()
                    .map(|f| (f.name.clone(), self.resolve_type(&f.ty)))
                    .collect();
                self.env.define_type(&s.name, Ty::Struct { name: s.name.clone(), fields });
            }
            Item::Enum(e) => {
                let variants: Vec<(String, super::types::VariantTy)> = e.variants.iter().map(|v| {
                    let vty = match &v.fields {
                        VariantKind::Unit => super::types::VariantTy::Unit,
                        VariantKind::Tuple(ts) => super::types::VariantTy::Tuple(
                            ts.iter().map(|t| self.resolve_type(t)).collect()
                        ),
                        VariantKind::Struct(fs) => super::types::VariantTy::Struct(
                            fs.iter().map(|f| (f.name.clone(), self.resolve_type(&f.ty))).collect()
                        ),
                    };
                    (v.name.clone(), vty)
                }).collect();
                self.env.define_type(&e.name, Ty::Enum { name: e.name.clone(), variants });
            }
            _ => {}
        }
    }

    fn check_item(&mut self, item: &Item) {
        match item {
            Item::Function(f) => self.check_function(f),
            Item::Impl(imp) => {
                for method in &imp.methods { self.check_function(method); }
            }
            _ => {}
        }
    }

    fn check_function(&mut self, f: &FnDecl) {
        // Comptime functions are interpreted, not compiled — skip type-checking their bodies
        if f.is_comptime {
            return;
        }

        let old_safety = self.current_safety.clone();
        self.current_safety = f.safety.clone();

        self.env.push_scope();

        // Bind parameters
        for p in &f.params {
            let ty = self.resolve_param_type(p);
            self.env.define(&p.name, ty, p.is_var);
        }

        let declared_ret = f.return_type.as_ref().map(|t| self.resolve_type(t)).unwrap_or(Ty::Unit);
        
        if let Some(req) = &f.requires {
            let req_ty = self.check_expr(req);
            if req_ty != Ty::Bool && req_ty != Ty::Error {
                self.error(f.span, "requires condition must be of type bool");
            }
        }
        
        if let Some(ens) = &f.ensures {
            // temporarily bind `result` for checking ensures
            self.env.push_scope();
            self.env.define("result", declared_ret.clone(), false);
            let ens_ty = self.check_expr(ens);
            if ens_ty != Ty::Bool && ens_ty != Ty::Error {
                self.error(f.span, "ensures condition must be of type bool");
            }
            self.env.pop_scope();
        }

        if let Some(body) = &f.body {
            let body_ty = self.check_block(body);
            // Check return type compatibility
            if body_ty != Ty::Unit && body_ty != Ty::Error && declared_ret != Ty::Unit {
                self.check_assignable(&declared_ret, &body_ty, f.span);
            }
        }

        self.env.pop_scope();
        self.current_safety = old_safety;
    }

    // ── Type resolution ──────────────────────────────────
    
    fn bind_pattern(&mut self, pattern: &Pattern, ty: &Ty) {
        match pattern {
            Pattern::Ident(name, _) => {
                if name != "_" {
                    self.env.define(name, ty.clone(), false);
                }
            }
            Pattern::Enum { path, variant, fields, .. } => {
                if let Ty::Enum { name: _enum_name, variants } = ty {
                    if let Some((_, v_ty)) = variants.iter().find(|(n, _)| n == variant) {
                        if let crate::typeck::types::VariantTy::Tuple(types) = v_ty {
                            for (i, field_pat) in fields.iter().enumerate() {
                                if i < types.len() {
                                    self.bind_pattern(field_pat, &types[i]);
                                }
                            }
                        }
                    }
                } else if path.len() == 1 {
                    // Look up enum by name
                    let enum_name = &path[0];
                    if let Some(Ty::Enum { variants, .. }) = self.env.lookup_type(enum_name).cloned() {
                        if let Some((_, v_ty)) = variants.iter().find(|(n, _)| n == variant) {
                            if let crate::typeck::types::VariantTy::Tuple(types) = v_ty {
                                for (i, field_pat) in fields.iter().enumerate() {
                                    if i < types.len() {
                                        self.bind_pattern(field_pat, &types[i]);
                                    }
                                }
                            }
                        }
                    }
                }
            }
            Pattern::Tuple(pats, _) => {
                if let Ty::Tuple(types) = ty {
                    for (p, t) in pats.iter().zip(types.iter()) {
                        self.bind_pattern(p, t);
                    }
                }
            }
            _ => {}
        }
    }

    fn resolve_type(&mut self, ty: &TypeExpr) -> Ty {
        match ty {
            TypeExpr::Path(path, _) => {
                let name = path.join(".");
                Ty::from_name(&name)
                    .or_else(|| self.env.lookup_type(&name).cloned())
                    .unwrap_or(Ty::Named(name))
            }
            TypeExpr::Applied { base, args, .. } => {
                let name = base.join(".");
                match name.as_str() {
                    "Option" => {
                        let inner = if !args.is_empty() { self.resolve_type(&args[0]) } else { Ty::Error };
                        Ty::Option(Box::new(inner))
                    }
                    "Result" => {
                        let t = if args.len() > 0 { self.resolve_type(&args[0]) } else { Ty::Error };
                        let e = if args.len() > 1 { self.resolve_type(&args[1]) } else { Ty::Error };
                        Ty::Result(Box::new(t), Box::new(e))
                    }
                    "Vec" => {
                        let inner = if !args.is_empty() { self.resolve_type(&args[0]) } else { Ty::Error };
                        Ty::Slice(Box::new(inner)) // Vec as growable slice for now
                    }
                    _ => Ty::Named(name),
                }
            }
            TypeExpr::Reference { is_var, inner, .. } => {
                Ty::Reference { is_var: *is_var, inner: Box::new(self.resolve_type(inner)) }
            }
            TypeExpr::Pointer { is_var, inner, .. } => {
                Ty::Pointer { is_var: *is_var, inner: Box::new(self.resolve_type(inner)) }
            }
            TypeExpr::Array { element, .. } => {
                Ty::Array(Box::new(self.resolve_type(element)), 0) // size deferred
            }
            TypeExpr::Slice { element, .. } => {
                Ty::Slice(Box::new(self.resolve_type(element)))
            }
            TypeExpr::Tuple(ts, _) => {
                Ty::Tuple(ts.iter().map(|t| self.resolve_type(t)).collect())
            }
            TypeExpr::Fn { params, ret, .. } => {
                let ps: Vec<Ty> = params.iter().map(|t| self.resolve_type(t)).collect();
                let r = ret.as_ref().map(|t| self.resolve_type(t)).unwrap_or(Ty::Unit);
                Ty::Function { params: ps, ret: Box::new(r) }
            }
            TypeExpr::Option(inner, _) => Ty::Option(Box::new(self.resolve_type(inner))),
            TypeExpr::Dyn(inner, _) => self.resolve_type(inner), // simplified
            TypeExpr::Comptime(inner, _) => self.resolve_type(inner),
        }
    }

    fn resolve_param_type(&mut self, p: &Param) -> Ty {
        p.ty.as_ref().map(|t| self.resolve_type(t)).unwrap_or_else(|| self.fresh_infer())
    }

    // ── Block & statement checking ───────────────────────

    fn check_block(&mut self, block: &Block) -> Ty {
        let old_safety = self.current_safety.clone();
        if let Some(s) = &block.safety {
            self.current_safety = s.clone();
        }
        
        self.env.push_scope();
        for stmt in &block.stmts {
            self.check_stmt(stmt);
        }
        let ty = if let Some(expr) = &block.trailing_expr {
            self.check_expr(expr)
        } else {
            Ty::Unit
        };
        self.env.pop_scope();
        
        self.current_safety = old_safety;
        ty
    }

    fn check_stmt(&mut self, stmt: &Stmt) {
        self.current_stmt_borrows.clear();
        match stmt {
            Stmt::Let(l) => {
                let val_ty = self.check_expr(&l.value);
                let declared = l.ty.as_ref().map(|t| self.resolve_type(t));
                let ty = if let Some(dec) = declared {
                    self.check_assignable(&dec, &val_ty, l.span);
                    dec
                } else {
                    val_ty
                };
                if let Pattern::Ident(name, _) = &l.pattern {
                    self.env.define(name, ty.clone(), false);
                } else {
                    self.bind_pattern(&l.pattern, &ty);
                }
                
                // Transfer temporary borrows to the scope (owned by this new variable)
                let borrows = std::mem::take(&mut self.current_stmt_borrows);
                for (b_name, b_path, is_mut) in borrows {
                    self.env.add_scope_borrow(&b_name, b_path, is_mut);
                }
            }
            Stmt::Var(v) => {
                let ty = if let Some(val) = &v.value {
                    let val_ty = self.check_expr(val);
                    if let Some(dec) = v.ty.as_ref().map(|t| self.resolve_type(t)) {
                        self.check_assignable(&dec, &val_ty, v.span);
                        dec
                    } else {
                        val_ty
                    }
                } else {
                    v.ty.as_ref().map(|t| self.resolve_type(t)).unwrap_or(Ty::Error)
                };
                self.env.define(&v.name, ty, true);
                
                // Transfer temporary borrows
                let borrows = std::mem::take(&mut self.current_stmt_borrows);
                for (b_name, b_path, is_mut) in borrows {
                    self.env.add_scope_borrow(&b_name, b_path, is_mut);
                }
            }
            Stmt::Assign(a) => {
                if let Expr::Ident(name, span) = &a.target {
                    if let Some(binding) = self.env.lookup(name) {
                        if !binding.is_mutable {
                            self.error(*span, &format!("cannot assign to immutable variable '{}'", name));
                        }
                    }
                }
                let target_ty = self.check_expr(&a.target);
                let value_ty = self.check_expr(&a.value);
                self.check_assignable(&target_ty, &value_ty, a.span);
                self.release_temp_borrows();
            }
            Stmt::Expr(e) => { 
                self.check_expr(&e.expr); 
                self.release_temp_borrows();
            }
            Stmt::Return(r) => {
                if let Some(val) = &r.value { self.check_expr(val); }
                self.release_temp_borrows();
            }
            Stmt::Break(_) | Stmt::Continue(_) => {}
            Stmt::Defer(d) => { 
                self.check_expr(&d.body); 
                self.release_temp_borrows();
            }
        }
    }

    fn release_temp_borrows(&mut self) {
        let borrows = std::mem::take(&mut self.current_stmt_borrows);
        for (b_name, b_path, is_mut) in borrows {
            self.env.release_borrow(&b_name, &b_path, is_mut);
        }
    }

    fn extract_path(&self, expr: &Expr) -> Option<(String, Vec<String>)> {
        match expr {
            Expr::Ident(name, _) => Some((name.clone(), Vec::new())),
            Expr::FieldAccess { object, field, .. } => {
                if let Some((root, mut path)) = self.extract_path(object) {
                    path.push(field.clone());
                    Some((root, path))
                } else {
                    None
                }
            }
            _ => None,
        }
    }

    // ── Expression checking ──────────────────────────────

    fn check_expr(&mut self, expr: &Expr) -> Ty {
        match expr {
            Expr::IntLiteral(_, _) => Ty::I32,   // default int type
            Expr::FloatLiteral(_, _) => Ty::F64,  // default float type
            Expr::StringLiteral(_, _) => Ty::String,
            Expr::BoolLiteral(_, _) => Ty::Bool,
            Expr::CharLiteral(_, _) => Ty::Char,

            Expr::Ident(name, span) => {
                let mut is_moved = false;
                let mut is_mut_borrowed = false;
                let mut is_fn = false;
                let mut ty = Ty::Error;

                if let Some(binding) = self.env.lookup(name) {
                    ty = binding.ty.clone();
                    match binding.state {
                        super::env::VarState::Moved(_) => is_moved = true,
                        super::env::VarState::BorrowedMutably(_) => is_mut_borrowed = true,
                        _ => {}
                    }
                } else if let Some(fn_ty) = self.env.lookup_fn(name) {
                    ty = fn_ty.clone();
                    is_fn = true;
                } else if let Some(t) = self.env.lookup_type(name) {
                    ty = t.clone();
                    is_fn = true;
                } else {
                    self.error(*span, &format!("undefined variable '{}'", name));
                    return Ty::Error;
                }

                if is_moved {
                    self.error(*span, &format!("use of moved value '{}'", name));
                } else if is_mut_borrowed {
                    self.error(*span, &format!("cannot use '{}' because it is borrowed mutably", name));
                } else if !is_fn {
                    let is_copy = ty.is_numeric() || ty == Ty::Bool || ty == Ty::Char 
                        || ty == Ty::String || matches!(ty, Ty::Named(_));
                    if !is_copy {
                        if let Some(b) = self.env.lookup_mut(name) {
                            b.state = super::env::VarState::Moved(*span);
                        }
                    }
                }
                ty
            }
            Expr::SelfValue(_) => self.fresh_infer(), // resolved during impl checking

            Expr::Binary { left, op, right, span } => {
                let lt = self.check_expr(left);
                let rt = self.check_expr(right);
                self.check_binary_op(op, &lt, &rt, *span)
            }
            Expr::Unary { op, operand, span } => {
                // Special handling for borrowing to avoid moving the operand
                if let UnaryOp::Ref | UnaryOp::RefMut = op {
                    if let Some((name, path)) = self.extract_path(operand.as_ref()) {
                        let is_mut = matches!(op, UnaryOp::RefMut);
                        let binding_info = self.env.lookup(&name).map(|b| (b.is_mutable, b.state.clone(), b.ty.clone()));
                        
                        if let Some((is_mutable, state, ty)) = binding_info {
                            if is_mut && !is_mutable {
                                self.error(*span, &format!("cannot borrow immutable variable '{}' as mutable", name));
                            }
                            let path_state = state.get_path(&path).clone();
                            match path_state {
                                super::env::VarState::Moved(prev) => {
                                    self.error_rich(crate::error::CompilerError {
                                        category: crate::error::ErrorCategory::Ownership,
                                        severity: crate::error::Severity::Error,
                                        span: *span,
                                        code: "E0382".to_string(),
                                        message: format!("cannot borrow moved value '{}'", name),
                                        explanation: Some("The value was already moved, so its original memory is invalid.".to_string()),
                                        suggestion: None,
                                        related: vec![crate::error::RelatedSpan { span: prev, description: "moved here".to_string() }],
                                    });
                                }
                                super::env::VarState::BorrowedMutably(prev) => {
                                    self.error_rich(crate::error::CompilerError {
                                        category: crate::error::ErrorCategory::Ownership,
                                        severity: crate::error::Severity::Error,
                                        span: *span,
                                        code: "E0201".to_string(),
                                        message: format!("cannot borrow `{}` as {} — already borrowed mutably", name, if is_mut { "mutable" } else { "immutable" }),
                                        explanation: Some("You cannot borrow a value while a mutable borrow is active. This prevents data races and iterator invalidation.".to_string()),
                                        suggestion: None,
                                        related: vec![crate::error::RelatedSpan { span: prev, description: "mutable borrow starts here".to_string() }],
                                    });
                                }
                                super::env::VarState::BorrowedImmutably(spans) if is_mut => {
                                    let mut related = Vec::new();
                                    for s in &spans {
                                        related.push(crate::error::RelatedSpan { span: *s, description: "immutable borrow starts here".to_string() });
                                    }
                                    self.error_rich(crate::error::CompilerError {
                                        category: crate::error::ErrorCategory::Ownership,
                                        severity: crate::error::Severity::Error,
                                        span: *span,
                                        code: "E0201".to_string(),
                                        message: format!("cannot borrow `{}` as mutable — already borrowed as immutable", name),
                                        explanation: Some("You cannot take a mutable reference while immutable references exist.".to_string()),
                                        suggestion: None,
                                        related,
                                    });
                                }
                                _ => {}
                            }
                            
                            if let Some(b) = self.env.lookup_mut(&name) {
                                if is_mut { b.state.set_path(&path, super::env::VarState::BorrowedMutably(*span)); }
                                else { 
                                    if let super::env::VarState::BorrowedImmutably(mut spans) = b.state.get_path(&path).clone() {
                                        spans.push(*span);
                                        b.state.set_path(&path, super::env::VarState::BorrowedImmutably(spans));
                                    } else {
                                        b.state.set_path(&path, super::env::VarState::BorrowedImmutably(vec![*span])); 
                                    }
                                }
                            }
                            self.current_stmt_borrows.push((name, path, is_mut));
                            return Ty::Reference { is_var: is_mut, inner: Box::new(ty) };
                        }
                    }
                }
                let t = self.check_expr(operand);
                self.check_unary_op(op, &t, *span)
            }

            Expr::Call { callee, args, span } => {
                let callee_ty = self.check_expr(callee);
                
                // Enforce Safety Boundaries
                let callee_fn_name = match callee.as_ref() {
                    Expr::Ident(name, _) => Some(name.clone()),
                    Expr::Path(path, _) => Some(path.join(".")),
                    _ => None,
                };
                if let Some(fn_name) = callee_fn_name {
                    if let Some(decl) = self.functions.get(&fn_name) {
                        let required_safety = &decl.safety;
                        match (&self.current_safety, required_safety) {
                            (SafetyMode::Safe, SafetyMode::Raw) => {
                                self.error(*span, &format!("cannot call `raw` function '{}' from `safe` code. Wrap it in a `raw {{ ... }}` block.", fn_name));
                            }
                            (SafetyMode::Safe, SafetyMode::Trusted) => {
                                self.error(*span, &format!("cannot call `trusted` function '{}' from `safe` code. Wrap it in a `trusted {{ ... }}` block.", fn_name));
                            }
                            (SafetyMode::Trusted, SafetyMode::Raw) => {
                                self.error(*span, &format!("cannot call `raw` function '{}' from `trusted` code. Wrap it in a `raw {{ ... }}` block.", fn_name));
                            }
                            _ => {} // Allowed (Raw -> anything, Trusted -> Trusted/Safe, Safe -> Safe)
                        }
                    }
                }

                let arg_tys: Vec<Ty> = args.iter().map(|a| self.check_expr(&a.value)).collect();
                match &callee_ty {
                    Ty::Function { params, ret } => {
                        if params.len() != arg_tys.len() {
                            self.error(*span, &format!(
                                "function expects {} arguments, got {}", params.len(), arg_tys.len()
                            ));
                        } else {
                            for (i, (p, a)) in params.iter().zip(arg_tys.iter()).enumerate() {
                                if !self.types_compatible(p, a) {
                                    self.error(*span, &format!(
                                        "argument {} type mismatch: expected {}, got {}",
                                        i + 1, p.display(), a.display()
                                    ));
                                }
                            }
                        }
                        *ret.clone()
                    }
                    Ty::Error => Ty::Error,
                    _ => {
                        // Might be a constructor or comptime call — allow for now
                        self.fresh_infer()
                    }
                }
            }

            Expr::MethodCall { receiver, method: _, args, span: _ } => {
                let _recv_ty = self.check_expr(receiver);
                if let Ty::Enum { name, .. } = &_recv_ty {
                    for a in args { self.check_expr(&a.value); }
                    return Ty::Named(name.clone());
                }
                for a in args { self.check_expr(&a.value); }
                // Method resolution is complex — return inferred for now
                self.fresh_infer()
            }

            Expr::FieldAccess { object, field, span } => {
                let obj_ty = self.check_expr(object);
                if let Ty::Enum { name, .. } = &obj_ty {
                    return Ty::Named(name.clone());
                }
                match &obj_ty {
                    Ty::Struct { fields, .. } => {
                        if let Some((_, fty)) = fields.iter().find(|(n, _)| n == field) {
                            fty.clone()
                        } else {
                            self.error(*span, &format!("no field '{}' on type {}", field, obj_ty.display()));
                            Ty::Error
                        }
                    }
                    _ => self.fresh_infer(), // defer to later pass
                }
            }

            Expr::Index { object, index, span } => {
                let obj_ty = self.check_expr(object);
                let idx_ty = self.check_expr(index);
                if !idx_ty.is_integer() && idx_ty != Ty::Error {
                    self.error(*span, &format!("index must be integer, got {}", idx_ty.display()));
                }
                match &obj_ty {
                    Ty::Array(inner, _) | Ty::Slice(inner) => *inner.clone(),
                    Ty::String => Ty::Char,
                    _ => self.fresh_infer(),
                }
            }

            Expr::ErrorPropagate { expr: inner, .. } => {
                let t = self.check_expr(inner);
                match &t {
                    Ty::Result(ok, _) => *ok.clone(),
                    Ty::Option(inner) => *inner.clone(),
                    _ => t,
                }
            }
            Expr::Unwrap { expr: inner, .. } => {
                let t = self.check_expr(inner);
                match &t {
                    Ty::Option(inner) => *inner.clone(),
                    Ty::Result(ok, _) => *ok.clone(),
                    _ => t,
                }
            }

            Expr::If { condition, then_block, else_ifs, else_block, span, .. } => {
                let cond_ty = self.check_expr(condition);
                if cond_ty != Ty::Bool && cond_ty != Ty::Error {
                    self.error(*span, &format!("if condition must be bool, got {}", cond_ty.display()));
                }
                let then_ty = self.check_block(then_block);
                
                for (cond, block) in else_ifs {
                    let c_ty = self.check_expr(cond);
                    if c_ty != Ty::Bool && c_ty != Ty::Error {
                        self.error(*span, &format!("else if condition must be bool, got {}", c_ty.display()));
                    }
                    self.check_block(block);
                }
                
                if let Some(else_b) = else_block {
                    let else_ty = self.check_block(else_b);
                    if then_ty != else_ty && then_ty != Ty::Error && else_ty != Ty::Error {
                        // Different branch types — could warn, but allow for now
                    }
                    then_ty
                } else {
                    Ty::Unit
                }
            }

            Expr::Match { subject, arms, .. } => {
                let subject_ty = self.check_expr(subject);
                let mut result_ty = Ty::Unit;
                for arm in arms {
                    self.env.push_scope();
                    self.bind_pattern(&arm.pattern, &subject_ty);
                    let arm_ty = self.check_expr(&arm.body);
                    self.env.pop_scope();
                    result_ty = arm_ty;
                }
                result_ty
            }

            Expr::For { iterator, body, .. } => {
                self.check_expr(iterator);
                self.check_block(body);
                Ty::Unit
            }
            Expr::While { condition, body, span, .. } => {
                let cond = self.check_expr(condition);
                if cond != Ty::Bool && cond != Ty::Error {
                    self.error(*span, "while condition must be bool");
                }
                self.check_block(body);
                Ty::Unit
            }
            Expr::Loop { body, .. } => { self.check_block(body); Ty::Never }
            Expr::Block(block) => self.check_block(block),
            Expr::Spawn { body, .. } => { self.check_block(body); Ty::Unit }
            Expr::Comptime { body, span: _ } => {
                let mut evaluator = crate::comptime::Evaluator::new(&self.functions);
                let value = evaluator.eval_expr(body);
                
                if evaluator.has_errors() {
                    for err in evaluator.errors {
                        self.error_rich(err);
                    }
                    Ty::Error
                } else {
                    match value {
                        crate::comptime::Value::Int(_) => Ty::I32,
                        crate::comptime::Value::Float(_) => Ty::F64,
                        crate::comptime::Value::String(_) => Ty::String,
                        crate::comptime::Value::Bool(_) => Ty::Bool,
                        crate::comptime::Value::Char(_) => Ty::Char,
                        crate::comptime::Value::Unit => Ty::Unit,
                        // Comptime generics: the comptime block returned an actual type
                        crate::comptime::Value::Type(ty) => ty,
                        crate::comptime::Value::Array(items) => {
                            // Infer element type from first element
                            let elem_ty = if let Some(first) = items.first() {
                                match first {
                                    crate::comptime::Value::Int(_) => Ty::I32,
                                    crate::comptime::Value::Float(_) => Ty::F64,
                                    crate::comptime::Value::String(_) => Ty::String,
                                    crate::comptime::Value::Bool(_) => Ty::Bool,
                                    crate::comptime::Value::Char(_) => Ty::Char,
                                    _ => self.fresh_infer(),
                                }
                            } else {
                                self.fresh_infer()
                            };
                            Ty::Array(Box::new(elem_ty), items.len())
                        }
                    }
                }
            }
            Expr::Array(items, _) => {
                if items.is_empty() {
                    Ty::Array(Box::new(self.fresh_infer()), 0)
                } else {
                    let first = self.check_expr(&items[0]);
                    for item in &items[1..] {
                        let t = self.check_expr(item);
                        // Could check all same type
                        let _ = t;
                    }
                    Ty::Array(Box::new(first), items.len())
                }
            }
            Expr::Tuple(items, _) => {
                Ty::Tuple(items.iter().map(|e| self.check_expr(e)).collect())
            }

            Expr::Closure { body, .. } => self.check_expr(body),
            Expr::StructLiteral { .. } => self.fresh_infer(),
            Expr::Range { .. } => self.fresh_infer(),
            Expr::Pipe { left, right, .. } => { self.check_expr(left); self.check_expr(right) }
            Expr::Path(_, _) => self.fresh_infer(),
        }
    }

    // ── Operator checking ────────────────────────────────

    fn check_binary_op(&mut self, op: &BinOp, left: &Ty, right: &Ty, span: crate::lexer::Span) -> Ty {
        match op {
            BinOp::Add | BinOp::Sub | BinOp::Mul | BinOp::Div | BinOp::Mod => {
                if left.is_numeric() && right.is_numeric() {
                    if left == right { left.clone() }
                    else { self.error(span, &format!("mismatched types in arithmetic: {} and {}", left.display(), right.display())); Ty::Error }
                } else if *left == Ty::Error || *right == Ty::Error { Ty::Error }
                else { self.error(span, &format!("cannot apply arithmetic to {} and {}", left.display(), right.display())); Ty::Error }
            }
            BinOp::Eq | BinOp::NotEq | BinOp::Less | BinOp::Greater | BinOp::LessEq | BinOp::GreaterEq => {
                Ty::Bool
            }
            BinOp::And | BinOp::Or => {
                if *left != Ty::Bool && *left != Ty::Error {
                    self.error(span, &format!("logical operator requires bool, got {}", left.display()));
                }
                Ty::Bool
            }
            BinOp::BitAnd | BinOp::BitOr | BinOp::BitXor | BinOp::Shl | BinOp::Shr => {
                if left.is_integer() { left.clone() }
                else if *left == Ty::Error { Ty::Error }
                else { self.error(span, &format!("bitwise op requires integer, got {}", left.display())); Ty::Error }
            }
        }
    }

    fn check_unary_op(&mut self, op: &UnaryOp, operand: &Ty, span: crate::lexer::Span) -> Ty {
        match op {
            UnaryOp::Neg => {
                if operand.is_numeric() { operand.clone() }
                else if *operand == Ty::Error { Ty::Error }
                else { self.error(span, &format!("cannot negate {}", operand.display())); Ty::Error }
            }
            UnaryOp::Not => {
                if *operand == Ty::Bool { Ty::Bool }
                else if *operand == Ty::Error { Ty::Error }
                else { self.error(span, &format!("'!' requires bool, got {}", operand.display())); Ty::Error }
            }
            UnaryOp::BitNot => {
                if operand.is_integer() { operand.clone() }
                else { self.error(span, &format!("'~' requires integer, got {}", operand.display())); Ty::Error }
            }
            UnaryOp::Ref => Ty::Reference { is_var: false, inner: Box::new(operand.clone()) },
            UnaryOp::RefMut => Ty::Reference { is_var: true, inner: Box::new(operand.clone()) },
            UnaryOp::Deref => {
                match operand {
                    Ty::Reference { inner, .. } | Ty::Pointer { inner, .. } => *inner.clone(),
                    _ => { self.error(span, &format!("cannot dereference {}", operand.display())); Ty::Error }
                }
            }
        }
    }

    // ── Type compatibility ───────────────────────────────

    fn types_compatible(&self, expected: &Ty, actual: &Ty) -> bool {
        if expected == actual { return true; }
        matches!((expected, actual),
            (Ty::Error, _) | (_, Ty::Error) |
            (Ty::Infer(_), _) | (_, Ty::Infer(_)) |
            (Ty::Named(_), _) | (_, Ty::Named(_))
        )
    }

    fn check_assignable(&mut self, expected: &Ty, actual: &Ty, span: crate::lexer::Span) {
        if !self.types_compatible(expected, actual) {
            self.error(span, &format!("type mismatch: expected {}, got {}", expected.display(), actual.display()));
        }
    }
}
