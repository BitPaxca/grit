use crate::lexer::Span;
use super::types::TypeExpr;
use super::stmt::{Expr, Stmt};

/// Top-level items in a Grit source file
#[derive(Debug)]
pub enum Item {
    Import(ImportDecl),
    Function(FnDecl),
    Struct(StructDecl),
    Enum(EnumDecl),
    Trait(TraitDecl),
    Impl(ImplBlock),
    Const(ConstDecl),
    TypeAlias(TypeAlias),
    ExternBlock(ExternBlock),
}

#[derive(Debug)]
pub struct ImportDecl {
    pub path: Vec<String>,
    pub names: Option<Vec<String>>, // None = import whole module
    pub span: Span,
}

#[derive(Debug)]
pub struct FnDecl {
    pub name: String,
    pub is_pub: bool,
    pub is_comptime: bool,
    pub is_extern: bool,
    pub params: Vec<Param>,
    pub return_type: Option<Box<TypeExpr>>,
    pub body: Option<Block>,
    pub span: Span,
}

#[derive(Debug)]
pub struct Param {
    pub name: String,
    pub ty: Option<Box<TypeExpr>>,
    pub is_var: bool,
    pub is_owned: bool,
    pub is_self: bool,
    pub span: Span,
}

#[derive(Debug)]
pub struct StructDecl {
    pub name: String,
    pub is_pub: bool,
    pub fields: Vec<StructField>,
    pub span: Span,
}

#[derive(Debug)]
pub struct StructField {
    pub name: String,
    pub ty: TypeExpr,
    pub is_pub: bool,
    pub span: Span,
}

#[derive(Debug)]
pub struct EnumDecl {
    pub name: String,
    pub is_pub: bool,
    pub variants: Vec<EnumVariant>,
    pub span: Span,
}

#[derive(Debug)]
pub struct EnumVariant {
    pub name: String,
    pub fields: VariantKind,
    pub span: Span,
}

#[derive(Debug)]
pub enum VariantKind {
    Unit,
    Tuple(Vec<TypeExpr>),
    Struct(Vec<StructField>),
}

#[derive(Debug)]
pub struct TraitDecl {
    pub name: String,
    pub is_pub: bool,
    pub super_traits: Vec<String>,
    pub methods: Vec<FnDecl>,
    pub span: Span,
}

#[derive(Debug)]
pub struct ImplBlock {
    pub target: String,
    pub trait_name: Option<String>,
    pub methods: Vec<FnDecl>,
    pub span: Span,
}

#[derive(Debug)]
pub struct ConstDecl {
    pub name: String,
    pub is_pub: bool,
    pub ty: TypeExpr,
    pub value: Expr,
    pub span: Span,
}

#[derive(Debug)]
pub struct TypeAlias {
    pub name: String,
    pub is_pub: bool,
    pub ty: TypeExpr,
    pub span: Span,
}

#[derive(Debug)]
pub struct ExternBlock {
    pub functions: Vec<FnDecl>,
    pub span: Span,
}

/// A block of statements, optionally ending with an expression
#[derive(Debug)]
pub struct Block {
    pub stmts: Vec<Stmt>,
    pub trailing_expr: Option<Box<Expr>>,
    pub span: Span,
}
