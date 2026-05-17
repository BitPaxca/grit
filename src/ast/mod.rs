mod expr;
mod stmt;
mod types;
mod items;

pub use expr::*;
pub use stmt::*;
pub use types::*;
pub use items::*;

use crate::lexer::Span;

/// A unique ID for each AST node — used for later passes (type checking, etc.)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct NodeId(pub u32);

/// A complete Grit source file
#[derive(Debug)]
pub struct SourceFile {
    pub items: Vec<Item>,
}
