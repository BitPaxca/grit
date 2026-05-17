use crate::lexer::Span;

/// Type expressions
#[derive(Debug, Clone)]
pub enum TypeExpr {
    /// Simple named type or path: i32, Vec, std.io.File
    Path(Vec<String>, Span),

    /// Generic/comptime application: Vec(i32), HashMap(String, i32)
    Applied {
        base: Vec<String>,
        args: Vec<TypeExpr>,
        span: Span,
    },

    /// Reference: &T or &var T
    Reference {
        is_var: bool,
        inner: Box<TypeExpr>,
        span: Span,
    },

    /// Pointer: *T or *var T
    Pointer {
        is_var: bool,
        inner: Box<TypeExpr>,
        span: Span,
    },

    /// Array: [T; N]
    Array {
        element: Box<TypeExpr>,
        size: Box<super::stmt::Expr>,
        span: Span,
    },

    /// Slice: [T]
    Slice {
        element: Box<TypeExpr>,
        span: Span,
    },

    /// Tuple: (T1, T2, T3)
    Tuple(Vec<TypeExpr>, Span),

    /// Function type: fn(i32, i32) -> i32
    Fn {
        params: Vec<TypeExpr>,
        ret: Option<Box<TypeExpr>>,
        span: Span,
    },

    /// Option shorthand: T?
    Option(Box<TypeExpr>, Span),

    /// Dynamic trait: dyn Trait
    Dyn(Box<TypeExpr>, Span),

    /// Comptime type
    Comptime(Box<TypeExpr>, Span),
}
