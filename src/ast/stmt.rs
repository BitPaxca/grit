use crate::lexer::Span;
use super::items::Block;
use super::types::TypeExpr;

/// Statements — things that don't produce a value (or whose value is discarded)
#[derive(Debug)]
pub enum Stmt {
    Let(LetStmt),
    Var(VarStmt),
    Assign(AssignStmt),
    Expr(ExprStmt),
    Return(ReturnStmt),
    Break(BreakStmt),
    Continue(ContinueStmt),
    Defer(DeferStmt),
}

#[derive(Debug)]
pub struct LetStmt {
    pub pattern: Pattern,
    pub ty: Option<Box<TypeExpr>>,
    pub value: Expr,
    pub span: Span,
}

#[derive(Debug)]
pub struct VarStmt {
    pub name: String,
    pub ty: Option<Box<TypeExpr>>,
    pub value: Option<Box<Expr>>,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub enum AssignOp {
    Eq, PlusEq, MinusEq, StarEq, SlashEq, PercentEq,
    AmpEq, PipeEq, CaretEq, ShlEq, ShrEq,
}

#[derive(Debug)]
pub struct AssignStmt {
    pub target: Expr,
    pub op: AssignOp,
    pub value: Expr,
    pub span: Span,
}

#[derive(Debug)]
pub struct ExprStmt {
    pub expr: Expr,
    pub span: Span,
}

#[derive(Debug)]
pub struct ReturnStmt {
    pub value: Option<Box<Expr>>,
    pub span: Span,
}

#[derive(Debug)]
pub struct BreakStmt {
    pub label: Option<String>,
    pub value: Option<Box<Expr>>,
    pub span: Span,
}

#[derive(Debug)]
pub struct ContinueStmt {
    pub label: Option<String>,
    pub span: Span,
}

#[derive(Debug)]
pub struct DeferStmt {
    pub body: Expr,
    pub span: Span,
}

/// Patterns for destructuring in let, match, for, etc.
#[derive(Debug)]
pub enum Pattern {
    Ident(String, Span),
    Wildcard(Span),
    Literal(Box<Expr>),
    Tuple(Vec<Pattern>, Span),
    Struct {
        path: Vec<String>,
        fields: Vec<(String, Option<Pattern>)>,
        rest: bool, // has ..
        span: Span,
    },
    Enum {
        path: Vec<String>,
        variant: String,
        fields: Vec<Pattern>,
        span: Span,
    },
    Ref {
        is_var: bool,
        inner: Box<Pattern>,
        span: Span,
    },
    Var(Box<Pattern>, Span),
}

/// Expressions — things that produce a value
#[derive(Debug)]
pub enum Expr {
    // Literals
    IntLiteral(u128, Span),
    FloatLiteral(f64, Span),
    StringLiteral(String, Span),
    BoolLiteral(bool, Span),
    CharLiteral(char, Span),

    // Identifiers & paths
    Ident(String, Span),
    SelfValue(Span),
    Path(Vec<String>, Span),

    // Binary & unary ops
    Binary {
        left: Box<Expr>,
        op: BinOp,
        right: Box<Expr>,
        span: Span,
    },
    Unary {
        op: UnaryOp,
        operand: Box<Expr>,
        span: Span,
    },

    // Postfix
    Call {
        callee: Box<Expr>,
        args: Vec<CallArg>,
        span: Span,
    },
    MethodCall {
        receiver: Box<Expr>,
        method: String,
        args: Vec<CallArg>,
        span: Span,
    },
    FieldAccess {
        object: Box<Expr>,
        field: String,
        span: Span,
    },
    Index {
        object: Box<Expr>,
        index: Box<Expr>,
        span: Span,
    },
    ErrorPropagate {
        expr: Box<Expr>,
        span: Span,
    },
    Unwrap {
        expr: Box<Expr>,
        span: Span,
    },

    // Control flow (all are expressions in Grit)
    If {
        condition: Box<Expr>,
        then_block: Block,
        else_ifs: Vec<(Expr, Block)>,
        else_block: Option<Block>,
        span: Span,
    },
    Match {
        subject: Box<Expr>,
        arms: Vec<MatchArm>,
        span: Span,
    },
    For {
        label: Option<String>,
        pattern: Box<Pattern>,
        iterator: Box<Expr>,
        body: Block,
        span: Span,
    },
    While {
        label: Option<String>,
        condition: Box<Expr>,
        body: Block,
        span: Span,
    },
    Loop {
        label: Option<String>,
        body: Block,
        span: Span,
    },
    Block(Block),

    // Spawn
    Spawn {
        kind: SpawnKind,
        body: Block,
        span: Span,
    },

    // Comptime
    Comptime {
        body: Box<Expr>,
        span: Span,
    },

    // Closures
    Closure {
        params: Vec<super::items::Param>,
        body: Box<Expr>,
        span: Span,
    },

    // Array & tuple literals
    Array(Vec<Expr>, Span),
    Tuple(Vec<Expr>, Span),

    // Struct literal
    StructLiteral {
        path: Vec<String>,
        fields: Vec<FieldInit>,
        span: Span,
    },

    // Range
    Range {
        start: Option<Box<Expr>>,
        end: Option<Box<Expr>>,
        span: Span,
    },

    // Pipe operator
    Pipe {
        left: Box<Expr>,
        right: Box<Expr>,
        span: Span,
    },
}

#[derive(Debug)]
pub struct CallArg {
    pub name: Option<String>,
    pub value: Expr,
    pub span: Span,
}

#[derive(Debug)]
pub struct FieldInit {
    pub name: String,
    pub value: Option<Expr>, // None = shorthand (name == local var)
    pub is_spread: bool,     // ..expr
    pub span: Span,
}

#[derive(Debug)]
pub struct MatchArm {
    pub pattern: Pattern,
    pub guard: Option<Box<Expr>>,
    pub body: Expr,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub enum BinOp {
    Add, Sub, Mul, Div, Mod,
    And, Or,
    BitAnd, BitOr, BitXor, Shl, Shr,
    Eq, NotEq, Less, Greater, LessEq, GreaterEq,
}

#[derive(Debug, Clone, PartialEq)]
pub enum UnaryOp {
    Neg,       // -
    Not,       // !
    BitNot,    // ~
    Ref,       // &
    RefMut,    // &var
    Deref,     // *
}

#[derive(Debug, Clone, PartialEq)]
pub enum SpawnKind {
    Task,
    Thread,
}
