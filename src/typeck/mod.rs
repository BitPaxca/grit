pub mod types;
mod env;
mod checker;
#[cfg(test)]
mod tests;

pub use checker::TypeChecker;
