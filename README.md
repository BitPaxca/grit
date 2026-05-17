# Grit Programming Language

Grit is a next-generation systems programming language designed to replace C and Rust as the gold standard for low-level, high-performance software. It features zero garbage collection, an LLVM backend, and a philosophy of being raw, honest, and uncompromising.

## Core Philosophy
- **Raw**: Unrestricted access to hardware, pointers, and memory layout.
- **Honest**: No hidden control flow, no implicit allocations, no hidden runtime.
- **Simple**: A simpler ownership model than Rust without sacrificing safety.

## Current Status
Grit is currently in active development. 

The compiler (`gritc`) is written in Rust and has successfully bootstrapped the following phases:
1. **Lexer**: Tokenization with advanced error tracking.
2. **Parser**: Pratt-parsing recursive descent AST generation.
3. **Typechecker**: Comprehensive static type verification and inference.
4. **Codegen**: Preliminary C-transpiler and LLVM IR framework.

## Usage
To run the compiler on a `.gr` file:
```bash
cargo run -- docs/examples/hello.gr --run
```

### Example: Hello World
```rust
// hello.gr
fn main() {
    print("Hello, Grit.")
}
```
