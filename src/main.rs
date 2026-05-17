mod lexer;
mod ast;
mod parser;
mod typeck;
mod codegen;
pub mod error;
pub mod comptime;
pub mod concurrency;
pub mod stdlib;
pub mod smt;

use std::env;
use std::fs;
use std::process;
use std::path::Path;

fn main() {
    let args: Vec<String> = env::args().collect();

    if args.len() < 2 {
        eprintln!("gritc: no input file");
        eprintln!("usage: gritc <file.gr> [--emit-c | --emit-llvm | --run]");
        process::exit(1);
    }

    let filename = &args[1];
    let emit_c = args.contains(&"--emit-c".to_string());
    let emit_llvm = args.contains(&"--emit-llvm".to_string());
    let run_mode = args.contains(&"--run".to_string());

    let source = match fs::read_to_string(filename) {
        Ok(s) => s,
        Err(e) => { eprintln!("gritc: cannot read '{}': {}", filename, e); process::exit(1); }
    };

    // Phase 1: Lex
    let mut lex = lexer::Lexer::new(&source, filename);
    let tokens = lex.tokenize();
    if lex.has_errors() {
        let reporter = error::ErrorReporter::new(&source, filename);
        for err in lex.errors.iter() { reporter.report(err); }
        process::exit(1);
    }

    // Phase 2: Parse
    let mut par = parser::Parser::new(tokens);
    let program = par.parse_program();
    if par.has_errors() {
        let reporter = error::ErrorReporter::new(&source, filename);
        for err in par.errors.iter() { reporter.report(err); }
        process::exit(1);
    }

    // Phase 3: Type check
    let mut checker = typeck::TypeChecker::new();
    checker.check_program(&program);
    if checker.has_errors() {
        let reporter = error::ErrorReporter::new(&source, filename);
        for err in checker.errors.iter() {
            reporter.report(err);
        }
        process::exit(1);
    }

    // Phase 4: SMT Solver Proofs
    let mut prover = smt::SmtProver::new();
    if let Some(proof_script) = prover.generate_proofs(&program) {
        let stem = Path::new(filename).file_stem().unwrap().to_str().unwrap();
        let proof_file = format!("{}_proof.smt2", stem);
        fs::write(&proof_file, &proof_script).expect("failed to write SMT proof file");
        println!("gritc: generated SMT proofs to '{}'", proof_file);
    }

    // Phase 5: Concurrency verification
    let mut conc = concurrency::ConcurrencyVerifier::new();
    conc.check_program(&program);
    // Print warnings even if no errors
    if !conc.warnings.is_empty() {
        let reporter = error::ErrorReporter::new(&source, filename);
        for w in conc.warnings.iter() {
            reporter.report(w);
        }
    }
    if conc.has_errors() {
        let reporter = error::ErrorReporter::new(&source, filename);
        for err in conc.errors.iter() {
            reporter.report(err);
        }
        process::exit(1);
    }

    // Phase 6: Codegen
    let stem = Path::new(filename).file_stem().unwrap().to_str().unwrap();

    if emit_llvm {
        let mut emitter = codegen::LLVMEmitter::new();
        let ir = emitter.emit_program(&program);
        let out_file = format!("{}.ll", stem);
        fs::write(&out_file, &ir).expect("failed to write .ll file");
        println!("gritc: wrote LLVM IR to '{}'", out_file);
        return;
    }

    // Default: emit C, compile, optionally run
    let mut emitter = codegen::CEmitter::new();
    let c_code = emitter.emit_program(&program);
    let c_file = format!("{}.c", stem);
    fs::write(&c_file, &c_code).expect("failed to write .c file");

    if emit_c {
        println!("gritc: wrote C code to '{}'", c_file);
        return;
    }

    // Try to compile the C file
    let exe_file = format!("{}.exe", stem);

    // Use the VS Build Tools developer environment to invoke cl.exe
    let bat_file = format!("{}_build.bat", stem);
    let compile_cmd = format!(
        "@echo off\r\ncall \"C:\\Program Files (x86)\\Microsoft Visual Studio\\2022\\BuildTools\\VC\\Auxiliary\\Build\\vcvars64.bat\" >nul 2>&1\r\ncl.exe /nologo /Fe:\"{}\" \"{}\"",
        exe_file, c_file
    );
    fs::write(&bat_file, &compile_cmd).expect("failed to write .bat file");

    let result = process::Command::new("cmd.exe")
        .arg("/C")
        .arg(&bat_file)
        .output();

    let _ = fs::remove_file(&bat_file);

    let compiled = match &result {
        Ok(output) => output.status.success(),
        Err(_) => false,
    };

    if !compiled {
        println!("gritc: C code written to '{}' (no C compiler found to compile it)", c_file);
        println!("       To compile manually: cl.exe {} /Fe:{}", c_file, exe_file);
        return;
    }

    // Clean up (disabled for debugging)
    // let _ = fs::remove_file(&c_file);
    println!("gritc: compiled '{}' -> '{}'", filename, exe_file);

    if run_mode {
        let status = process::Command::new(format!(".\\{}", exe_file))
            .status()
            .expect("failed to run compiled binary");

        process::exit(status.code().unwrap_or(1));
    }
}
