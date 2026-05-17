/// Source location tracking — every token knows exactly where it came from.
/// This is critical for legendary error messages.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Span {
    /// Byte offset of the start of this token in the source
    pub start: usize,
    /// Byte offset of the end of this token (exclusive)
    pub end: usize,
    /// Line number (1-indexed)
    pub line: u32,
    /// Column number (1-indexed, in characters not bytes)
    pub col: u32,
}

impl Span {
    pub fn new(start: usize, end: usize, line: u32, col: u32) -> Self {
        Self { start, end, line, col }
    }

    pub fn len(&self) -> usize {
        self.end - self.start
    }
}

/// A single token produced by the lexer.
#[derive(Debug, Clone, PartialEq)]
pub struct Token {
    pub kind: TokenKind,
    pub span: Span,
    /// The raw source text of this token
    pub lexeme: String,
}

impl Token {
    pub fn new(kind: TokenKind, span: Span, lexeme: String) -> Self {
        Self { kind, span, lexeme }
    }
}

/// Every possible token in the Grit language.
#[derive(Debug, Clone, PartialEq)]
pub enum TokenKind {
    // ── Literals ──────────────────────────────────────────
    IntLiteral(u128),
    FloatLiteral(f64),
    StringLiteral(String),
    ByteStringLiteral(Vec<u8>),
    CharLiteral(char),
    True,
    False,

    // ── Identifier ───────────────────────────────────────
    Ident,

    // ── Keywords ─────────────────────────────────────────
    And,
    Break,
    Comptime,
    Const,
    Continue,
    Defer,
    Dyn,
    Else,
    Ensures,
    Enum,
    Extern,
    Fn,
    For,
    If,
    Impl,
    Import,
    In,
    Let,
    Loop,
    Match,
    Or,
    Owned,
    Pub,
    Raw,
    Requires,
    Return,
    Safe,
    SelfValue,  // `self` as a value
    Spawn,
    Struct,
    Task,
    Thread,
    Trait,
    Trusted,
    Type,
    Var,
    Where,
    While,

    // ── Arithmetic Operators ─────────────────────────────
    Plus,       // +
    Minus,      // -
    Star,       // *
    Slash,      // /
    Percent,    // %

    // ── Bitwise Operators ────────────────────────────────
    Ampersand,  // &
    Pipe,       // |
    Caret,      // ^
    Tilde,      // ~
    Shl,        // <<
    Shr,        // >>

    // ── Comparison Operators ─────────────────────────────
    EqEq,       // ==
    BangEq,     // !=
    Less,       // <
    Greater,    // >
    LessEq,     // <=
    GreaterEq,  // >=

    // ── Assignment Operators ─────────────────────────────
    Eq,         // =
    PlusEq,     // +=
    MinusEq,    // -=
    StarEq,     // *=
    SlashEq,    // /=
    PercentEq,  // %=
    AmpEq,      // &=
    PipeEq,     // |=
    CaretEq,    // ^=
    ShlEq,      // <<=
    ShrEq,      // >>=

    // ── Delimiters ───────────────────────────────────────
    LParen,     // (
    RParen,     // )
    LBrace,     // {
    RBrace,     // }
    LBracket,   // [
    RBracket,   // ]

    // ── Punctuation ──────────────────────────────────────
    Comma,      // ,
    Dot,        // .
    DotDot,     // ..
    Colon,      // :
    Semicolon,  // ;
    Arrow,      // ->
    FatArrow,   // =>
    Question,   // ?
    Bang,       // !
    PipeArrow,  // |>
    Hash,       // #

    // ── Special ──────────────────────────────────────────
    Newline,    // significant newlines (statement terminators)
    Eof,

    // ── Error Recovery ───────────────────────────────────
    Error(String),
}

impl TokenKind {
    /// Look up a keyword from an identifier string.
    /// Returns None if the string is not a keyword.
    pub fn keyword(s: &str) -> Option<TokenKind> {
        match s {
            "and"      => Some(TokenKind::And),
            "break"    => Some(TokenKind::Break),
            "comptime" => Some(TokenKind::Comptime),
            "const"    => Some(TokenKind::Const),
            "continue" => Some(TokenKind::Continue),
            "defer"    => Some(TokenKind::Defer),
            "dyn"      => Some(TokenKind::Dyn),
            "else"     => Some(TokenKind::Else),
            "ensures"  => Some(TokenKind::Ensures),
            "enum"     => Some(TokenKind::Enum),
            "extern"   => Some(TokenKind::Extern),
            "false"    => Some(TokenKind::False),
            "fn"       => Some(TokenKind::Fn),
            "for"      => Some(TokenKind::For),
            "if"       => Some(TokenKind::If),
            "impl"     => Some(TokenKind::Impl),
            "import"   => Some(TokenKind::Import),
            "in"       => Some(TokenKind::In),
            "let"      => Some(TokenKind::Let),
            "loop"     => Some(TokenKind::Loop),
            "match"    => Some(TokenKind::Match),
            "or"       => Some(TokenKind::Or),
            "owned"    => Some(TokenKind::Owned),
            "pub"      => Some(TokenKind::Pub),
            "raw"      => Some(TokenKind::Raw),
            "requires" => Some(TokenKind::Requires),
            "return"   => Some(TokenKind::Return),
            "safe"     => Some(TokenKind::Safe),
            "self"     => Some(TokenKind::SelfValue),
            "spawn"    => Some(TokenKind::Spawn),
            "struct"   => Some(TokenKind::Struct),
            "task"     => Some(TokenKind::Task),
            "thread"   => Some(TokenKind::Thread),
            "trait"    => Some(TokenKind::Trait),
            "true"     => Some(TokenKind::True),
            "trusted"  => Some(TokenKind::Trusted),
            "type"     => Some(TokenKind::Type),
            "var"      => Some(TokenKind::Var),
            "where"    => Some(TokenKind::Where),
            "while"    => Some(TokenKind::While),
            _          => None,
        }
    }

    /// Returns true if this token kind can end a statement
    /// (used for automatic semicolon / newline insertion logic)
    pub fn ends_statement(&self) -> bool {
        matches!(
            self,
            TokenKind::Ident
                | TokenKind::IntLiteral(_)
                | TokenKind::FloatLiteral(_)
                | TokenKind::StringLiteral(_)
                | TokenKind::CharLiteral(_)
                | TokenKind::True
                | TokenKind::False
                | TokenKind::Return
                | TokenKind::Break
                | TokenKind::Continue
                | TokenKind::RParen
                | TokenKind::RBracket
                | TokenKind::RBrace
                | TokenKind::Question
                | TokenKind::Bang
                | TokenKind::SelfValue
        )
    }
}
