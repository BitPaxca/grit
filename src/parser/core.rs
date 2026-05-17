use crate::lexer::{Token, TokenKind, Span};
use crate::ast::*;

pub struct Parser {
    tokens: Vec<Token>,
    pos: usize,
    pub errors: Vec<crate::error::CompilerError>,
}

impl Parser {
    pub fn new(tokens: Vec<Token>) -> Self {
        Self { tokens, pos: 0, errors: Vec::new() }
    }

    pub fn has_errors(&self) -> bool { !self.errors.is_empty() }
    
    pub fn error_strings(&self) -> Vec<String> {
        self.errors.iter().map(|e| e.message.clone()).collect()
    }

    fn error(&mut self, msg: &str) {
        let span = self.current_span();
        self.errors.push(crate::error::CompilerError {
            category: crate::error::ErrorCategory::Syntax,
            severity: crate::error::Severity::Error,
            span,
            code: "E0102".to_string(), // generic parse error
            message: msg.to_string(),
            explanation: None,
            suggestion: None,
            related: Vec::new(),
        });
    }

    // ── Token helpers ────────────────────────────────────

    fn peek(&self) -> &TokenKind {
        &self.tokens[self.pos].kind
    }

    fn current_span(&self) -> Span {
        self.tokens[self.pos].span
    }

    fn current_lexeme(&self) -> &str {
        &self.tokens[self.pos].lexeme
    }

    fn at_end(&self) -> bool {
        matches!(self.peek(), TokenKind::Eof)
    }

    fn advance(&mut self) -> &Token {
        let tok = &self.tokens[self.pos];
        if !self.at_end() {
            self.pos += 1;
        }
        tok
    }

    fn check(&self, kind: &TokenKind) -> bool {
        std::mem::discriminant(self.peek()) == std::mem::discriminant(kind)
    }

    fn eat(&mut self, kind: &TokenKind) -> bool {
        if self.check(kind) {
            self.advance();
            true
        } else {
            false
        }
    }

    fn expect(&mut self, kind: &TokenKind) -> Result<&Token, ()> {
        if self.check(kind) {
            Ok(self.advance())
        } else {
            self.error(&format!("expected {:?}, found {:?}", kind, self.peek()));
            Err(())
        }
    }

    fn expect_ident(&mut self) -> Result<String, ()> {
        if matches!(self.peek(), TokenKind::Ident) {
            Ok(self.advance().lexeme.clone())
        } else {
            self.error(&format!("expected identifier, found {:?}", self.peek()));
            Err(())
        }
    }

    fn skip_newlines(&mut self) {
        while matches!(self.peek(), TokenKind::Newline) {
            self.advance();
        }
    }

    fn expect_newline(&mut self) {
        if !self.eat(&TokenKind::Newline) && !self.at_end() && !self.check(&TokenKind::RBrace) {
            // Don't error — just continue
        }
    }

    pub fn parse_program(&mut self) -> SourceFile {
        self.skip_newlines();
        let mut items = Vec::new();
        while !self.at_end() {
            match self.parse_item() {
                Ok(item) => items.push(item),
                Err(_) => {
                    while !self.at_end() && !matches!(self.peek(), TokenKind::Newline) {
                        self.advance();
                    }
                    self.skip_newlines();
                }
            }
        }
        SourceFile { items }
    }

    fn parse_item(&mut self) -> Result<Item, ()> {
        self.skip_newlines();
        if self.at_end() { return Err(()); }
        let is_pub = self.eat(&TokenKind::Pub);
        
        // Check for safety modifiers
        let safety = if self.eat(&TokenKind::Trusted) {
            SafetyMode::Trusted
        } else if self.eat(&TokenKind::Raw) {
            SafetyMode::Raw
        } else if self.eat(&TokenKind::Safe) {
            SafetyMode::Safe
        } else {
            SafetyMode::Safe // default
        };

        match self.peek() {
            TokenKind::Import => self.parse_import(),
            TokenKind::Fn => self.parse_function(is_pub, false, false, safety),
            TokenKind::Struct => self.parse_struct(is_pub),
            TokenKind::Enum => self.parse_enum(is_pub),
            TokenKind::Trait => self.parse_trait(is_pub),
            TokenKind::Impl => self.parse_impl(),
            TokenKind::Const => self.parse_const(is_pub),
            TokenKind::Type => self.parse_type_alias(is_pub),
            TokenKind::Extern => self.parse_extern(is_pub),
            TokenKind::Comptime => { self.advance(); self.parse_function(is_pub, true, false, safety) }
            _ => { self.error(&format!("expected item, found {:?}", self.peek())); Err(()) }
        }
    }

    fn parse_import(&mut self) -> Result<Item, ()> {
        let span = self.current_span();
        self.expect(&TokenKind::Import)?;
        let mut path = vec![self.expect_ident()?];
        while self.eat(&TokenKind::Dot) {
            if self.check(&TokenKind::LBrace) { break; }
            path.push(self.expect_ident()?);
        }
        let names = if self.eat(&TokenKind::LBrace) {
            let mut n = vec![self.expect_ident()?];
            while self.eat(&TokenKind::Comma) {
                if self.check(&TokenKind::RBrace) { break; }
                n.push(self.expect_ident()?);
            }
            self.expect(&TokenKind::RBrace)?;
            Some(n)
        } else { None };
        self.expect_newline();
        Ok(Item::Import(ImportDecl { path, names, span }))
    }

    pub fn parse_function(&mut self, is_pub: bool, is_comptime: bool, is_extern: bool, safety: SafetyMode) -> Result<Item, ()> {
        let span = self.current_span();
        self.expect(&TokenKind::Fn)?;
        let name = self.expect_ident()?;
        self.expect(&TokenKind::LParen)?;
        let params = self.parse_param_list()?;
        self.expect(&TokenKind::RParen)?;
        let return_type = if self.eat(&TokenKind::Arrow) { Some(Box::new(self.parse_type()?)) } else { None };
        
        self.skip_newlines();
        let requires = if self.eat(&TokenKind::Requires) {
            Some(Box::new(self.parse_expr()?))
        } else { None };
        
        self.skip_newlines();
        let ensures = if self.eat(&TokenKind::Ensures) {
            Some(Box::new(self.parse_expr()?))
        } else { None };

        self.skip_newlines();
        let body = if self.check(&TokenKind::LBrace) { Some(self.parse_block(Some(safety.clone()))?) } else { self.expect_newline(); None };
        Ok(Item::Function(FnDecl { name, is_pub, is_comptime, is_extern, safety, params, return_type, requires, ensures, body, span }))
    }

    fn parse_param_list(&mut self) -> Result<Vec<Param>, ()> {
        let mut params = Vec::new();
        if self.check(&TokenKind::RParen) { return Ok(params); }
        params.push(self.parse_param()?);
        while self.eat(&TokenKind::Comma) {
            if self.check(&TokenKind::RParen) { break; }
            params.push(self.parse_param()?);
        }
        Ok(params)
    }

    fn parse_param(&mut self) -> Result<Param, ()> {
        let span = self.current_span();
        let is_var = self.eat(&TokenKind::Var);
        let is_owned = if !is_var { self.eat(&TokenKind::Owned) } else { false };
        let is_self = matches!(self.peek(), TokenKind::SelfValue);
        let name = if is_self { self.advance(); "self".to_string() } else { self.expect_ident()? };
        let ty = if self.eat(&TokenKind::Colon) { Some(Box::new(self.parse_type()?)) } else { None };
        Ok(Param { name, ty, is_var, is_owned, is_self, span })
    }

    fn parse_struct(&mut self, is_pub: bool) -> Result<Item, ()> {
        let span = self.current_span();
        self.expect(&TokenKind::Struct)?;
        let name = self.expect_ident()?;
        self.expect(&TokenKind::LBrace)?;
        self.skip_newlines();
        let mut fields = Vec::new();
        while !self.check(&TokenKind::RBrace) && !self.at_end() {
            let fp = self.eat(&TokenKind::Pub);
            let fs = self.current_span();
            let fn_ = self.expect_ident()?;
            self.expect(&TokenKind::Colon)?;
            let ft = self.parse_type()?;
            fields.push(StructField { name: fn_, ty: ft, is_pub: fp, span: fs });
            self.skip_newlines();
        }
        self.expect(&TokenKind::RBrace)?;
        self.skip_newlines();
        Ok(Item::Struct(StructDecl { name, is_pub, fields, span }))
    }

    fn parse_enum(&mut self, is_pub: bool) -> Result<Item, ()> {
        let span = self.current_span();
        self.expect(&TokenKind::Enum)?;
        let name = self.expect_ident()?;
        self.expect(&TokenKind::LBrace)?;
        self.skip_newlines();
        let mut variants = Vec::new();
        while !self.check(&TokenKind::RBrace) && !self.at_end() {
            let vs = self.current_span();
            let vn = self.expect_ident()?;
            let vf = if self.check(&TokenKind::LBrace) {
                self.advance(); self.skip_newlines();
                let mut fs = Vec::new();
                while !self.check(&TokenKind::RBrace) && !self.at_end() {
                    let s = self.current_span();
                    let n = self.expect_ident()?;
                    self.expect(&TokenKind::Colon)?;
                    let t = self.parse_type()?;
                    fs.push(StructField { name: n, ty: t, is_pub: false, span: s });
                    self.eat(&TokenKind::Comma); self.skip_newlines();
                }
                self.expect(&TokenKind::RBrace)?;
                VariantKind::Struct(fs)
            } else if self.eat(&TokenKind::LParen) {
                let mut ts = vec![self.parse_type()?];
                while self.eat(&TokenKind::Comma) { ts.push(self.parse_type()?); }
                self.expect(&TokenKind::RParen)?;
                VariantKind::Tuple(ts)
            } else { VariantKind::Unit };
            variants.push(EnumVariant { name: vn, fields: vf, span: vs });
            self.skip_newlines();
        }
        self.expect(&TokenKind::RBrace)?;
        self.skip_newlines();
        Ok(Item::Enum(EnumDecl { name, is_pub, variants, span }))
    }

    fn parse_trait(&mut self, is_pub: bool) -> Result<Item, ()> {
        let span = self.current_span();
        self.expect(&TokenKind::Trait)?;
        let name = self.expect_ident()?;
        let mut super_traits = Vec::new();
        if self.eat(&TokenKind::Colon) {
            super_traits.push(self.expect_ident()?);
            while self.eat(&TokenKind::Plus) { super_traits.push(self.expect_ident()?); }
        }
        self.expect(&TokenKind::LBrace)?; self.skip_newlines();
        let mut methods = Vec::new();
        while !self.check(&TokenKind::RBrace) && !self.at_end() {
            if let Ok(Item::Function(f)) = self.parse_function(false, false, false, SafetyMode::Safe) { methods.push(f); }
            self.skip_newlines();
        }
        self.expect(&TokenKind::RBrace)?; self.skip_newlines();
        Ok(Item::Trait(TraitDecl { name, is_pub, super_traits, methods, span }))
    }

    fn parse_impl(&mut self) -> Result<Item, ()> {
        let span = self.current_span();
        self.expect(&TokenKind::Impl)?;
        let first = self.expect_ident()?;
        let (target, trait_name) = if self.eat(&TokenKind::For) {
            (self.expect_ident()?, Some(first))
        } else { (first, None) };
        self.expect(&TokenKind::LBrace)?; self.skip_newlines();
        let mut methods = Vec::new();
        while !self.check(&TokenKind::RBrace) && !self.at_end() {
            if let Ok(Item::Function(f)) = self.parse_function(false, false, false, SafetyMode::Safe) { methods.push(f); }
            self.skip_newlines();
        }
        self.expect(&TokenKind::RBrace)?; self.skip_newlines();
        Ok(Item::Impl(ImplBlock { target, trait_name, methods, span }))
    }

    fn parse_const(&mut self, is_pub: bool) -> Result<Item, ()> {
        let span = self.current_span();
        self.expect(&TokenKind::Const)?;
        let name = self.expect_ident()?;
        self.expect(&TokenKind::Colon)?;
        let ty = self.parse_type()?;
        self.expect(&TokenKind::Eq)?;
        let value = self.parse_expr()?;
        self.expect_newline();
        Ok(Item::Const(ConstDecl { name, is_pub, ty, value, span }))
    }

    fn parse_type_alias(&mut self, is_pub: bool) -> Result<Item, ()> {
        let span = self.current_span();
        self.expect(&TokenKind::Type)?;
        let name = self.expect_ident()?;
        self.expect(&TokenKind::Eq)?;
        let ty = self.parse_type()?;
        self.expect_newline();
        Ok(Item::TypeAlias(TypeAlias { name, is_pub, ty, span }))
    }

    fn parse_extern(&mut self, is_pub: bool) -> Result<Item, ()> {
        let span = self.current_span();
        self.expect(&TokenKind::Extern)?;
        if self.check(&TokenKind::Fn) { return self.parse_function(is_pub, false, true, SafetyMode::Safe); }
        self.expect(&TokenKind::LBrace)?; self.skip_newlines();
        let mut functions = Vec::new();
        while !self.check(&TokenKind::RBrace) && !self.at_end() {
            if let Ok(Item::Function(f)) = self.parse_function(false, false, true, SafetyMode::Safe) { functions.push(f); }
            self.skip_newlines();
        }
        self.expect(&TokenKind::RBrace)?; self.skip_newlines();
        Ok(Item::ExternBlock(ExternBlock { functions, span }))
    }

    // ── Type parsing ─────────────────────────────────────

    pub fn parse_type(&mut self) -> Result<TypeExpr, ()> {
        let span = self.current_span();
        match self.peek() {
            TokenKind::Ampersand => {
                self.advance();
                let is_var = self.eat(&TokenKind::Var);
                let inner = self.parse_type()?;
                Ok(TypeExpr::Reference { is_var, inner: Box::new(inner), span })
            }
            TokenKind::Star => {
                self.advance();
                let is_var = self.eat(&TokenKind::Var);
                let inner = self.parse_type()?;
                Ok(TypeExpr::Pointer { is_var, inner: Box::new(inner), span })
            }
            TokenKind::LBracket => {
                self.advance();
                let elem = self.parse_type()?;
                if self.eat(&TokenKind::Semicolon) || self.eat(&TokenKind::Newline) {
                    // Check for semicolon as array separator
                    let size = self.parse_expr()?;
                    self.expect(&TokenKind::RBracket)?;
                    Ok(TypeExpr::Array { element: Box::new(elem), size: Box::new(size), span })
                } else {
                    self.expect(&TokenKind::RBracket)?;
                    Ok(TypeExpr::Slice { element: Box::new(elem), span })
                }
            }
            TokenKind::Fn => {
                self.advance();
                self.expect(&TokenKind::LParen)?;
                let mut params = Vec::new();
                if !self.check(&TokenKind::RParen) {
                    params.push(self.parse_type()?);
                    while self.eat(&TokenKind::Comma) { params.push(self.parse_type()?); }
                }
                self.expect(&TokenKind::RParen)?;
                let ret = if self.eat(&TokenKind::Arrow) { Some(Box::new(self.parse_type()?)) } else { None };
                Ok(TypeExpr::Fn { params, ret, span })
            }
            TokenKind::Dyn => {
                self.advance();
                let inner = self.parse_type()?;
                Ok(TypeExpr::Dyn(Box::new(inner), span))
            }
            TokenKind::Comptime => {
                self.advance();
                let inner = self.parse_type()?;
                Ok(TypeExpr::Comptime(Box::new(inner), span))
            }
            TokenKind::LParen => {
                self.advance();
                let first = self.parse_type()?;
                if self.eat(&TokenKind::Comma) {
                    let mut types = vec![first];
                    if !self.check(&TokenKind::RParen) {
                        types.push(self.parse_type()?);
                        while self.eat(&TokenKind::Comma) {
                            if self.check(&TokenKind::RParen) { break; }
                            types.push(self.parse_type()?);
                        }
                    }
                    self.expect(&TokenKind::RParen)?;
                    Ok(TypeExpr::Tuple(types, span))
                } else {
                    self.expect(&TokenKind::RParen)?;
                    Ok(first) // parenthesized type
                }
            }
            TokenKind::Ident => {
                let mut path = vec![self.expect_ident()?];
                while self.eat(&TokenKind::Dot) {
                    if !matches!(self.peek(), TokenKind::Ident) { break; }
                    path.push(self.expect_ident()?);
                }
                if self.check(&TokenKind::LParen) {
                    self.advance();
                    let mut args = Vec::new();
                    if !self.check(&TokenKind::RParen) {
                        args.push(self.parse_type()?);
                        while self.eat(&TokenKind::Comma) {
                            if self.check(&TokenKind::RParen) { break; }
                            args.push(self.parse_type()?);
                        }
                    }
                    self.expect(&TokenKind::RParen)?;
                    let ty = TypeExpr::Applied { base: path, args, span };
                    if self.eat(&TokenKind::Question) { Ok(TypeExpr::Option(Box::new(ty), span)) } else { Ok(ty) }
                } else {
                    let ty = TypeExpr::Path(path, span);
                    if self.eat(&TokenKind::Question) { Ok(TypeExpr::Option(Box::new(ty), span)) } else { Ok(ty) }
                }
            }
            TokenKind::Type => {
                self.advance();
                let ty = TypeExpr::Path(vec!["type".to_string()], span);
                Ok(ty)
            }
            _ => {
                self.error(&format!("expected type, found {:?}", self.peek()));
                Err(())
            }
        }
    }

    // ── Block parsing ────────────────────────────────────

    pub fn parse_block(&mut self, safety: Option<SafetyMode>) -> Result<Block, ()> {
        let span = self.current_span();
        self.expect(&TokenKind::LBrace)?;
        self.skip_newlines();
        let mut stmts = Vec::new();
        let mut trailing_expr = None;

        while !self.check(&TokenKind::RBrace) && !self.at_end() {
            // Try to parse as a statement
            match self.try_parse_stmt() {
                Ok(Some(stmt)) => {
                    stmts.push(stmt);
                    self.skip_newlines();
                }
                Ok(None) => {
                    // It's a trailing expression
                    let expr = self.parse_expr()?;
                    self.skip_newlines();
                    if self.check(&TokenKind::RBrace) {
                        trailing_expr = Some(Box::new(expr));
                    } else {
                        stmts.push(Stmt::Expr(ExprStmt { span: self.current_span(), expr }));
                        self.skip_newlines();
                    }
                }
                Err(_) => {
                    self.advance();
                    self.skip_newlines();
                }
            }
        }
        self.expect(&TokenKind::RBrace)?;
        Ok(Block { stmts, trailing_expr, safety, span })
    }

    fn try_parse_stmt(&mut self) -> Result<Option<Stmt>, ()> {
        match self.peek() {
            TokenKind::Let => { let s = self.parse_let()?; Ok(Some(s)) }
            TokenKind::Var => { let s = self.parse_var()?; Ok(Some(s)) }
            TokenKind::Return => { let s = self.parse_return()?; Ok(Some(s)) }
            TokenKind::Break => { let s = self.parse_break()?; Ok(Some(s)) }
            TokenKind::Continue => { let s = self.parse_continue()?; Ok(Some(s)) }
            TokenKind::Defer => { let s = self.parse_defer()?; Ok(Some(s)) }
            _ => {
                // Could be assignment or expression
                let expr = self.parse_expr()?;
                // Check for assignment operator
                let op = match self.peek() {
                    TokenKind::Eq => Some(AssignOp::Eq),
                    TokenKind::PlusEq => Some(AssignOp::PlusEq),
                    TokenKind::MinusEq => Some(AssignOp::MinusEq),
                    TokenKind::StarEq => Some(AssignOp::StarEq),
                    TokenKind::SlashEq => Some(AssignOp::SlashEq),
                    TokenKind::PercentEq => Some(AssignOp::PercentEq),
                    TokenKind::AmpEq => Some(AssignOp::AmpEq),
                    TokenKind::PipeEq => Some(AssignOp::PipeEq),
                    TokenKind::CaretEq => Some(AssignOp::CaretEq),
                    TokenKind::ShlEq => Some(AssignOp::ShlEq),
                    TokenKind::ShrEq => Some(AssignOp::ShrEq),
                    _ => None,
                };
                if let Some(op) = op {
                    let span = self.current_span();
                    self.advance();
                    let value = self.parse_expr()?;
                    self.expect_newline();
                    Ok(Some(Stmt::Assign(AssignStmt { target: expr, op, value, span })))
                } else {
                    // Check if newline follows — it's an expression statement
                    if self.eat(&TokenKind::Newline) || self.check(&TokenKind::RBrace) {
                        let span = self.current_span();
                        Ok(Some(Stmt::Expr(ExprStmt { expr, span })))
                    } else {
                        // Might be trailing expression
                        // Put it back conceptually — but we already consumed it
                        let span = self.current_span();
                        Ok(Some(Stmt::Expr(ExprStmt { expr, span })))
                    }
                }
            }
        }
    }

    fn parse_let(&mut self) -> Result<Stmt, ()> {
        let span = self.current_span();
        self.expect(&TokenKind::Let)?;
        let pattern = self.parse_pattern()?;
        let ty = if self.eat(&TokenKind::Colon) { Some(Box::new(self.parse_type()?)) } else { None };
        self.expect(&TokenKind::Eq)?;
        let value = self.parse_expr()?;
        self.expect_newline();
        Ok(Stmt::Let(LetStmt { pattern, ty, value, span }))
    }

    fn parse_var(&mut self) -> Result<Stmt, ()> {
        let span = self.current_span();
        self.expect(&TokenKind::Var)?;
        let name = self.expect_ident()?;
        let ty = if self.eat(&TokenKind::Colon) { Some(Box::new(self.parse_type()?)) } else { None };
        let value = if self.eat(&TokenKind::Eq) { Some(Box::new(self.parse_expr()?)) } else { None };
        self.expect_newline();
        Ok(Stmt::Var(VarStmt { name, ty, value, span }))
    }

    fn parse_return(&mut self) -> Result<Stmt, ()> {
        let span = self.current_span();
        self.expect(&TokenKind::Return)?;
        let value = if !self.check(&TokenKind::Newline) && !self.check(&TokenKind::RBrace) && !self.at_end() {
            Some(Box::new(self.parse_expr()?))
        } else { None };
        self.expect_newline();
        Ok(Stmt::Return(ReturnStmt { value, span }))
    }

    fn parse_break(&mut self) -> Result<Stmt, ()> {
        let span = self.current_span();
        self.expect(&TokenKind::Break)?;
        self.expect_newline();
        Ok(Stmt::Break(BreakStmt { label: None, value: None, span }))
    }

    fn parse_continue(&mut self) -> Result<Stmt, ()> {
        let span = self.current_span();
        self.expect(&TokenKind::Continue)?;
        self.expect_newline();
        Ok(Stmt::Continue(ContinueStmt { label: None, span }))
    }

    fn parse_defer(&mut self) -> Result<Stmt, ()> {
        let span = self.current_span();
        self.expect(&TokenKind::Defer)?;
        let body = self.parse_expr()?;
        self.expect_newline();
        Ok(Stmt::Defer(DeferStmt { body, span }))
    }

    fn parse_pattern(&mut self) -> Result<Pattern, ()> {
        let span = self.current_span();
        match self.peek().clone() {
            TokenKind::IntLiteral(n) => { self.advance(); Ok(Pattern::Literal(Box::new(Expr::IntLiteral(n, span)))) }
            TokenKind::StringLiteral(s) => { self.advance(); Ok(Pattern::Literal(Box::new(Expr::StringLiteral(s.clone(), span)))) }
            TokenKind::True => { self.advance(); Ok(Pattern::Literal(Box::new(Expr::BoolLiteral(true, span)))) }
            TokenKind::False => { self.advance(); Ok(Pattern::Literal(Box::new(Expr::BoolLiteral(false, span)))) }
            TokenKind::Ident => {
                let name = self.advance().lexeme.clone();
                
                // If it's a path like `EnumName.Variant`
                if self.eat(&TokenKind::Dot) {
                    let variant = self.expect_ident()?;
                    let mut fields = Vec::new();
                    if self.eat(&TokenKind::LParen) {
                        if !self.check(&TokenKind::RParen) {
                            fields.push(self.parse_pattern()?);
                            while self.eat(&TokenKind::Comma) {
                                if self.check(&TokenKind::RParen) { break; }
                                fields.push(self.parse_pattern()?);
                            }
                        }
                        self.expect(&TokenKind::RParen)?;
                    }
                    Ok(Pattern::Enum { path: vec![name], variant, fields, span })
                } else {
                    if name == "_" {
                        Ok(Pattern::Wildcard(span))
                    } else {
                        Ok(Pattern::Ident(name, span))
                    }
                }
            }
            _ => {
                self.error(&format!("expected pattern, found {:?}", self.peek()));
                Err(())
            }
        }
    }

    // ── Expression parsing (Pratt parser) ────────────────

    pub fn parse_expr(&mut self) -> Result<Expr, ()> {
        self.parse_or()
    }

    fn parse_or(&mut self) -> Result<Expr, ()> {
        let mut left = self.parse_and()?;
        while self.eat(&TokenKind::Or) {
            let span = self.current_span();
            let right = self.parse_and()?;
            left = Expr::Binary { left: Box::new(left), op: BinOp::Or, right: Box::new(right), span };
        }
        Ok(left)
    }

    fn parse_and(&mut self) -> Result<Expr, ()> {
        let mut left = self.parse_comparison()?;
        while self.eat(&TokenKind::And) {
            let span = self.current_span();
            let right = self.parse_comparison()?;
            left = Expr::Binary { left: Box::new(left), op: BinOp::And, right: Box::new(right), span };
        }
        Ok(left)
    }

    fn parse_comparison(&mut self) -> Result<Expr, ()> {
        let mut left = self.parse_bitor()?;
        loop {
            let op = match self.peek() {
                TokenKind::EqEq => BinOp::Eq,
                TokenKind::BangEq => BinOp::NotEq,
                TokenKind::Less => BinOp::Less,
                TokenKind::Greater => BinOp::Greater,
                TokenKind::LessEq => BinOp::LessEq,
                TokenKind::GreaterEq => BinOp::GreaterEq,
                _ => break,
            };
            let span = self.current_span();
            self.advance();
            let right = self.parse_bitor()?;
            left = Expr::Binary { left: Box::new(left), op, right: Box::new(right), span };
        }
        Ok(left)
    }

    fn parse_bitor(&mut self) -> Result<Expr, ()> {
        let mut left = self.parse_bitxor()?;
        while self.eat(&TokenKind::Pipe) {
            let span = self.current_span();
            let right = self.parse_bitxor()?;
            left = Expr::Binary { left: Box::new(left), op: BinOp::BitOr, right: Box::new(right), span };
        }
        Ok(left)
    }

    fn parse_bitxor(&mut self) -> Result<Expr, ()> {
        let mut left = self.parse_bitand()?;
        while self.eat(&TokenKind::Caret) {
            let span = self.current_span();
            let right = self.parse_bitand()?;
            left = Expr::Binary { left: Box::new(left), op: BinOp::BitXor, right: Box::new(right), span };
        }
        Ok(left)
    }

    fn parse_bitand(&mut self) -> Result<Expr, ()> {
        let mut left = self.parse_shift()?;
        while self.eat(&TokenKind::Ampersand) {
            let span = self.current_span();
            let right = self.parse_shift()?;
            left = Expr::Binary { left: Box::new(left), op: BinOp::BitAnd, right: Box::new(right), span };
        }
        Ok(left)
    }

    fn parse_shift(&mut self) -> Result<Expr, ()> {
        let mut left = self.parse_additive()?;
        loop {
            let op = match self.peek() {
                TokenKind::Shl => BinOp::Shl,
                TokenKind::Shr => BinOp::Shr,
                _ => break,
            };
            let span = self.current_span();
            self.advance();
            let right = self.parse_additive()?;
            left = Expr::Binary { left: Box::new(left), op, right: Box::new(right), span };
        }
        Ok(left)
    }

    fn parse_additive(&mut self) -> Result<Expr, ()> {
        let mut left = self.parse_multiplicative()?;
        loop {
            let op = match self.peek() {
                TokenKind::Plus => BinOp::Add,
                TokenKind::Minus => BinOp::Sub,
                _ => break,
            };
            let span = self.current_span();
            self.advance();
            let right = self.parse_multiplicative()?;
            left = Expr::Binary { left: Box::new(left), op, right: Box::new(right), span };
        }
        Ok(left)
    }

    fn parse_multiplicative(&mut self) -> Result<Expr, ()> {
        let mut left = self.parse_unary()?;
        loop {
            let op = match self.peek() {
                TokenKind::Star => BinOp::Mul,
                TokenKind::Slash => BinOp::Div,
                TokenKind::Percent => BinOp::Mod,
                _ => break,
            };
            let span = self.current_span();
            self.advance();
            let right = self.parse_unary()?;
            left = Expr::Binary { left: Box::new(left), op, right: Box::new(right), span };
        }
        Ok(left)
    }

    fn parse_unary(&mut self) -> Result<Expr, ()> {
        let span = self.current_span();
        match self.peek() {
            TokenKind::Minus => {
                self.advance();
                let operand = self.parse_unary()?;
                Ok(Expr::Unary { op: UnaryOp::Neg, operand: Box::new(operand), span })
            }
            TokenKind::Bang => {
                self.advance();
                let operand = self.parse_unary()?;
                Ok(Expr::Unary { op: UnaryOp::Not, operand: Box::new(operand), span })
            }
            TokenKind::Tilde => {
                self.advance();
                let operand = self.parse_unary()?;
                Ok(Expr::Unary { op: UnaryOp::BitNot, operand: Box::new(operand), span })
            }
            TokenKind::Ampersand => {
                self.advance();
                let is_var = self.eat(&TokenKind::Var);
                let operand = self.parse_unary()?;
                let op = if is_var { UnaryOp::RefMut } else { UnaryOp::Ref };
                Ok(Expr::Unary { op, operand: Box::new(operand), span })
            }
            TokenKind::Star => {
                self.advance();
                let operand = self.parse_unary()?;
                Ok(Expr::Unary { op: UnaryOp::Deref, operand: Box::new(operand), span })
            }
            _ => self.parse_postfix(),
        }
    }

    fn parse_postfix(&mut self) -> Result<Expr, ()> {
        let mut expr = self.parse_primary()?;
        loop {
            let span = self.current_span();
            match self.peek() {
                TokenKind::Dot => {
                    self.advance();
                    let field = self.expect_ident()?;
                    if self.check(&TokenKind::LParen) {
                        let args = self.parse_call_args()?;
                        expr = Expr::MethodCall { receiver: Box::new(expr), method: field, args, span };
                    } else {
                        expr = Expr::FieldAccess { object: Box::new(expr), field, span };
                    }
                }
                TokenKind::LParen => {
                    let args = self.parse_call_args()?;
                    expr = Expr::Call { callee: Box::new(expr), args, span };
                }
                TokenKind::LBracket => {
                    self.advance();
                    let index = self.parse_expr()?;
                    self.expect(&TokenKind::RBracket)?;
                    expr = Expr::Index { object: Box::new(expr), index: Box::new(index), span };
                }
                TokenKind::Question => {
                    self.advance();
                    expr = Expr::ErrorPropagate { expr: Box::new(expr), span };
                }
                TokenKind::Bang => {
                    self.advance();
                    expr = Expr::Unwrap { expr: Box::new(expr), span };
                }
                _ => break,
            }
        }
        Ok(expr)
    }

    fn parse_call_args(&mut self) -> Result<Vec<CallArg>, ()> {
        self.expect(&TokenKind::LParen)?;
        let mut args = Vec::new();
        if !self.check(&TokenKind::RParen) {
            args.push(self.parse_call_arg()?);
            while self.eat(&TokenKind::Comma) {
                if self.check(&TokenKind::RParen) { break; }
                args.push(self.parse_call_arg()?);
            }
        }
        self.expect(&TokenKind::RParen)?;
        Ok(args)
    }

    fn parse_call_arg(&mut self) -> Result<CallArg, ()> {
        let span = self.current_span();
        let value = self.parse_expr()?;
        Ok(CallArg { name: None, value, span })
    }

    fn parse_primary(&mut self) -> Result<Expr, ()> {
        let span = self.current_span();
        match self.peek().clone() {
            TokenKind::IntLiteral(n) => { self.advance(); Ok(Expr::IntLiteral(n, span)) }
            TokenKind::FloatLiteral(f) => { self.advance(); Ok(Expr::FloatLiteral(f, span)) }
            TokenKind::StringLiteral(ref s) => { let s = s.clone(); self.advance(); Ok(Expr::StringLiteral(s, span)) }
            TokenKind::CharLiteral(c) => { self.advance(); Ok(Expr::CharLiteral(c, span)) }
            TokenKind::True => { self.advance(); Ok(Expr::BoolLiteral(true, span)) }
            TokenKind::False => { self.advance(); Ok(Expr::BoolLiteral(false, span)) }
            TokenKind::SelfValue => { self.advance(); Ok(Expr::SelfValue(span)) }
            TokenKind::Ident => {
                let name = self.advance().lexeme.clone();
                Ok(Expr::Ident(name, span))
            }
            TokenKind::LParen => {
                self.advance();
                if self.check(&TokenKind::RParen) {
                    self.advance();
                    return Ok(Expr::Tuple(vec![], span));
                }
                let expr = self.parse_expr()?;
                if self.eat(&TokenKind::Comma) {
                    let mut items = vec![expr];
                    if !self.check(&TokenKind::RParen) {
                        items.push(self.parse_expr()?);
                        while self.eat(&TokenKind::Comma) {
                            if self.check(&TokenKind::RParen) { break; }
                            items.push(self.parse_expr()?);
                        }
                    }
                    self.expect(&TokenKind::RParen)?;
                    Ok(Expr::Tuple(items, span))
                } else {
                    self.expect(&TokenKind::RParen)?;
                    Ok(expr)
                }
            }
            TokenKind::LBracket => {
                self.advance();
                let mut items = Vec::new();
                if !self.check(&TokenKind::RBracket) {
                    items.push(self.parse_expr()?);
                    while self.eat(&TokenKind::Comma) {
                        if self.check(&TokenKind::RBracket) { break; }
                        items.push(self.parse_expr()?);
                    }
                }
                self.expect(&TokenKind::RBracket)?;
                Ok(Expr::Array(items, span))
            }
            TokenKind::If => self.parse_if_expr(),
            TokenKind::Match => self.parse_match_expr(),
            TokenKind::For => self.parse_for_expr(),
            TokenKind::While => self.parse_while_expr(),
            TokenKind::Loop => self.parse_loop_expr(),
            TokenKind::LBrace => {
                let block = self.parse_block(None)?;
                Ok(Expr::Block(block))
            }
            TokenKind::Safe => {
                self.advance();
                let block = self.parse_block(Some(SafetyMode::Safe))?;
                Ok(Expr::Block(block))
            }
            TokenKind::Trusted => {
                self.advance();
                let block = self.parse_block(Some(SafetyMode::Trusted))?;
                Ok(Expr::Block(block))
            }
            TokenKind::Raw => {
                self.advance();
                let block = self.parse_block(Some(SafetyMode::Raw))?;
                Ok(Expr::Block(block))
            }
            TokenKind::Comptime => {
                self.advance();
                let body = self.parse_expr()?;
                Ok(Expr::Comptime { body: Box::new(body), span })
            }
            TokenKind::Spawn => self.parse_spawn_expr(),
            _ => {
                self.error(&format!("expected expression, found {:?}", self.peek()));
                Err(())
            }
        }
    }

    fn parse_if_expr(&mut self) -> Result<Expr, ()> {
        let span = self.current_span();
        self.expect(&TokenKind::If)?;
        let condition = self.parse_expr()?;
        let then_block = self.parse_block(None)?;
        let mut else_ifs = Vec::new();
        let mut else_block = None;
        while self.eat(&TokenKind::Else) {
            if self.check(&TokenKind::If) {
                self.advance();
                let cond = self.parse_expr()?;
                let block = self.parse_block(None)?;
                else_ifs.push((cond, block));
            } else {
                else_block = Some(self.parse_block(None)?);
                break;
            }
        }
        Ok(Expr::If { condition: Box::new(condition), then_block, else_ifs, else_block, span })
    }

    fn parse_match_expr(&mut self) -> Result<Expr, ()> {
        let span = self.current_span();
        self.expect(&TokenKind::Match)?;
        let subject = self.parse_expr()?;
        self.expect(&TokenKind::LBrace)?;
        self.skip_newlines();
        let mut arms = Vec::new();
        while !self.check(&TokenKind::RBrace) && !self.at_end() {
            let a_span = self.current_span();
            let pattern = self.parse_pattern()?;
            let guard = if self.eat(&TokenKind::If) { Some(Box::new(self.parse_expr()?)) } else { None };
            self.expect(&TokenKind::FatArrow)?;
            let body = self.parse_expr()?;
            arms.push(MatchArm { pattern, guard, body, span: a_span });
            self.skip_newlines();
        }
        self.expect(&TokenKind::RBrace)?;
        Ok(Expr::Match { subject: Box::new(subject), arms, span })
    }

    fn parse_for_expr(&mut self) -> Result<Expr, ()> {
        let span = self.current_span();
        self.expect(&TokenKind::For)?;
        let pattern = self.parse_pattern()?;
        self.expect(&TokenKind::In)?;
        let iterator = self.parse_expr()?;
        let body = self.parse_block(None)?;
        Ok(Expr::For { label: None, pattern: Box::new(pattern), iterator: Box::new(iterator), body, span })
    }

    fn parse_while_expr(&mut self) -> Result<Expr, ()> {
        let span = self.current_span();
        self.expect(&TokenKind::While)?;
        let condition = self.parse_expr()?;
        let body = self.parse_block(None)?;
        Ok(Expr::While { label: None, condition: Box::new(condition), body, span })
    }

    fn parse_loop_expr(&mut self) -> Result<Expr, ()> {
        let span = self.current_span();
        self.expect(&TokenKind::Loop)?;
        let body = self.parse_block(None)?;
        Ok(Expr::Loop { label: None, body, span })
    }

    fn parse_spawn_expr(&mut self) -> Result<Expr, ()> {
        let span = self.current_span();
        self.expect(&TokenKind::Spawn)?;
        let kind = if self.eat(&TokenKind::Task) { SpawnKind::Task }
                   else if self.eat(&TokenKind::Thread) { SpawnKind::Thread }
                   else { self.error("expected 'task' or 'thread' after spawn"); return Err(()); };
                   
        let mut capabilities = Vec::new();
        if self.eat(&TokenKind::LBracket) {
            while !self.check(&TokenKind::RBracket) && !self.at_end() {
                let cap_span = self.current_span();
                let is_write = if self.check(&TokenKind::Ident) {
                    let lex = self.current_lexeme();
                    if lex == "read" { self.advance(); false }
                    else if lex == "write" { self.advance(); true }
                    else { self.error("expected 'read' or 'write' in capability list"); return Err(()); }
                } else {
                    self.error("expected 'read' or 'write' in capability list"); return Err(());
                };
                
                let var_name = self.expect_ident()?;
                let cap_kind = if is_write { CapabilityKind::Write } else { CapabilityKind::Read };
                capabilities.push(Capability { var_name, kind: cap_kind, span: cap_span });
                
                if !self.eat(&TokenKind::Comma) { break; }
            }
            self.expect(&TokenKind::RBracket)?;
        }
        
        let body = self.parse_block(None)?;
        Ok(Expr::Spawn { kind, capabilities, body, span })
    }
}
