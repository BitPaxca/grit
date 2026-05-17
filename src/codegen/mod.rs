mod c_emitter;
mod llvm_emitter;
#[cfg(test)]
mod tests;

pub use c_emitter::CEmitter;
pub use llvm_emitter::LLVMEmitter;
