# Grit Programming Language

A modern systems programming language with the speed of C, the safety of Rust, and compile-time mathematical proofs. Zero garbage collection.

## Why Grit?

```
fn process_payment(balance: i32, amount: i32) -> i32
    requires balance >= amount
    ensures result == balance - amount
{
    return balance - amount
}

fn main() {
    var funds = 5000

    spawn task [write funds] {
        funds = process_payment(funds, 1500)
        print_int(funds)
    }
}
```

**What you're looking at:**
- `requires` / `ensures` — compile-time SMT-solver contracts that mathematically prove your function is correct
- `spawn task [write funds]` — capability-based concurrency that prevents data races by requiring explicit permission grants
- No GC, no runtime — compiles to native machine code at the same speed as C

## Features

- **3-Tier Safety** — `safe` (default) → `trusted` (SMT-verified) → `raw` (C-level access)
- **Ownership & Borrowing** — Strict move semantics and borrow checking without a GC
- **SMT Contracts** — `requires` / `ensures` clauses generate formal proofs at compile-time
- **Capability Concurrency** — Spawned tasks must declare `[read x]` or `[write x]` to access outer state
- **Comptime Metaprogramming** — Execute normal functions at compile-time to generate types and constants
- **Standard Library** — Strings, Vectors, File I/O, Math, and more

## Quick Start

```bash
git clone https://github.com/BitPaxca/grit.git
cd grit
cargo build --release
cargo run --release -- docs/examples/hello.gr --run
```

### Hello World
```
fn main() {
    print("Hello, Grit!")
}
```

### Dynamic Strings & Vectors
```
fn main() {
    let greeting = string_concat("Hello, ", "world!")
    print(string_to_upper(greeting))

    var numbers = vec_new()
    vec_push(numbers, 42)
    vec_push(numbers, 99)
    print_int(vec_get(numbers, 0))
}
```

### File I/O
```
fn main() {
    write_file("output.txt", "Grit was here.")
    let data = read_file("output.txt")
    print(data)
    delete_file("output.txt")
}
```

## Documentation

📖 **[Language Guide](docs/guide.md)** — Full syntax reference, stdlib API, and example programs.

📁 **[Example Programs](docs/examples/)** — Ready-to-run `.gr` files.

## Compiler Flags

| Flag | Description |
|------|-------------|
| `--run` | Compile and immediately execute |
| `--emit-c` | Output the generated C code |
| `--emit-llvm` | Output LLVM IR (experimental) |

## Architecture

The `gritc` compiler pipeline:

```
.gr source → Lexer → Parser → Type Checker → SMT Prover → Concurrency Verifier → C Codegen → Native .exe
```

93 unit tests passing across all compiler phases.

## Performance

Grit compiles to optimized C, which is then compiled to native machine code. There is no garbage collector, no virtual machine, and no runtime overhead. Performance is identical to C and Rust.

## License

MIT
