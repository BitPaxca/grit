#[cfg(test)]
mod tests {
    use crate::lexer::Lexer;
    use crate::parser::Parser;
    use crate::typeck::TypeChecker;

    fn check(src: &str) -> Vec<String> {
        let mut lexer = Lexer::new(src, "test.gr");
        let tokens = lexer.tokenize();
        assert!(!lexer.has_errors(), "Lex errors: {:?}", lexer.error_strings());
        let mut parser = Parser::new(tokens);
        let program = parser.parse_program();
        assert!(!parser.has_errors(), "Parse errors: {:?}", parser.error_strings());
        let mut checker = TypeChecker::new();
        checker.check_program(&program);
        checker.error_strings()
    }

    fn check_err(src: &str) -> Vec<String> {
        let errs = check(src);
        if errs.is_empty() {
            panic!("Expected type errors but got none");
        }
        errs
    }

    fn check_ok(src: &str) {
        let errs = check(src);
        if !errs.is_empty() {
            panic!("Expected NO type errors, but got: {:?}", errs);
        }
    }

    #[test]
    fn test_hello_world() {
        check_ok(r#"fn main() {
    print("Hello, Grit.")
}"#);
    }

    #[test]
    fn test_let_binding_int() {
        check_ok("fn main() {\n    let x = 42\n}");
    }

    #[test]
    fn test_let_with_type() {
        check_ok("fn main() {\n    let x: i32 = 42\n}");
    }

    #[test]
    fn test_var_binding() {
        check_ok("fn main() {\n    var x = 10\n    x = 20\n}");
    }

    #[test]
    fn test_arithmetic() {
        check_ok("fn main() {\n    let x = 1 + 2\n    let y = x * 3\n}");
    }

    #[test]
    fn test_type_mismatch() {
        let errs = check_err("fn main() {\n    let x: bool = 42\n}");
        assert!(errs[0].contains("type mismatch"));
    }

    #[test]
    fn test_undefined_variable() {
        let errs = check_err("fn main() {\n    let x = y\n}");
        assert!(errs[0].contains("undefined variable"));
    }

    #[test]
    fn test_if_condition_bool() {
        check_ok("fn main() {\n    if true {\n        42\n    }\n}");
    }

    #[test]
    fn test_if_condition_not_bool() {
        let errs = check_err("fn main() {\n    if 42 {\n        0\n    }\n}");
        assert!(errs[0].contains("must be bool"));
    }

    #[test]
    fn test_function_call_args() {
        check_ok(r#"fn main() {
    print("hello")
}"#);
    }

    #[test]
    fn test_comparison_returns_bool() {
        check_ok("fn main() {\n    let x = 1 > 2\n}");
    }

    #[test]
    fn test_logical_ops() {
        check_ok("fn main() {\n    let x = true and false\n    let y = x or true\n}");
    }

    #[test]
    fn test_negation() {
        check_ok("fn main() {\n    let x = -42\n}");
    }

    #[test]
    fn test_not_on_nonbool() {
        let errs = check_err("fn main() {\n    let x = !42\n}");
        assert!(errs[0].contains("requires bool"));
    }

    #[test]
    fn test_reference() {
        check_ok("fn main() {\n    let x = 42\n    let y = &x\n}");
    }

    #[test]
    fn test_scoping() {
        let errs = check_err("fn main() {\n    if true {\n        let x = 42\n    }\n    let y = x\n}");
        assert!(errs[0].contains("undefined variable 'x'"));
    }

    #[test]
    fn test_immutable_assign() {
        let errs = check_err("fn main() {\n    let x = 42\n    x = 10\n}");
        assert!(errs[0].contains("cannot assign to immutable variable 'x'"));
    }

    #[test]
    fn test_use_after_move() {
        let errs = check_err("fn main() {\n    let s: String = \"test\"\n    let y = s\n    let z = s\n}");
        assert!(errs[0].contains("use of moved value 's'"));
    }

    #[test]
    fn test_mutable_borrow_immutable_var() {
        let errs = check_err("fn main() {\n    let x = 42\n    let y = &var x\n}");
        assert!(errs[0].contains("cannot borrow immutable variable 'x' as mutable"));
    }

    #[test]
    fn test_multiple_mutable_borrows() {
        let errs = check_err("fn main() {\n    var x = 42\n    let y = &var x\n    let z = &var x\n}");
        assert!(errs[0].contains("already borrowed mutably"));
    }

    #[test]
    fn test_read_while_mutably_borrowed() {
        let errs = check_err("fn main() {\n    var x = 42\n    let y = &var x\n    let z = x\n}");
        assert!(errs[0].contains("cannot use 'x' because it is borrowed mutably"));
    }

    #[test]
    fn test_borrow_lifetime_expires() {
        check_ok("fn main() {\n    var x = 42\n    {\n        let y = &var x\n    }\n    let z = &var x\n}");
    }

    #[test]
    fn test_immutable_aliasing() {
        check_ok("fn main() {\n    var x = 42\n    let y = &x\n    let z = &x\n}");
    }

    #[test]
    fn test_field_level_borrows() {
        check_ok("struct Point { a: i32 \n b: i32 }\nfn main(var v: Point) {\n    let ref_a = &var v.a\n    let ref_b = &var v.b\n}");
    }

    #[test]
    fn test_field_level_borrow_conflict() {
        let errs = check_err("struct Point { a: i32 \n b: i32 }\nfn main(var v: Point) {\n    let ref_a = &var v.a\n    let ref_a2 = &var v.a\n}");
        assert!(errs[0].contains("already borrowed mutably"));
    }

    #[test]
    fn test_comptime_success() {
        check_ok("fn main() {\n    let x = comptime 10 + 2 * 3\n}");
    }

    #[test]
    fn test_comptime_div_by_zero() {
        let errs = check_err("fn main() {\n    let x = comptime 10 / 0\n}");
        assert!(errs[0].contains("division by zero in comptime evaluation"));
    }

    #[test]
    fn test_comptime_block() {
        check_ok("fn main() {\n    let x = comptime {\n        let a = 10\n        let b = 20\n        if a < b {\n            a + b\n        } else {\n            a - b\n        }\n    }\n}");
    }

    #[test]
    fn test_comptime_function_call() {
        check_ok("fn compute(x: i32, y: i32) -> i32 {\n    if x > y {\n        return x * 2\n    }\n    y * 2\n}\n\nfn main() {\n    let x = comptime compute(5, 10)\n    let y = comptime compute(10, 5)\n}");
    }

    // ── Comptime Generics Tests ──────────────────────────

    #[test]
    fn test_comptime_type_value() {
        // A comptime block that returns a primitive type name resolves to that type
        check_ok("fn main() {\n    let T = comptime i32\n}");
    }

    #[test]
    fn test_comptime_type_generating_function() {
        // A comptime function that accepts a type and returns a type
        check_ok("comptime fn make_pair_type(T: type) -> type {\n    struct_type(\"Pair\", [\"first\", \"second\"], [T, T])\n}\n\nfn main() {\n    let PairI32 = comptime make_pair_type(i32)\n}");
    }

    #[test]
    fn test_comptime_array_of() {
        // Built-in array_of type constructor
        check_ok("fn main() {\n    let BufferType = comptime array_of(u8, 256)\n}");
    }

    #[test]
    fn test_comptime_ref_of() {
        // Built-in ref_of type constructor
        check_ok("fn main() {\n    let RefType = comptime ref_of(i32, false)\n}");
    }

    #[test]
    fn test_comptime_generic_struct_factory() {
        // Simulate the Vec(T) pattern from the master design
        check_ok("comptime fn Container(T: type) -> type {\n    struct_type(\"Container\", [\"data\", \"len\", \"cap\"], [T, usize, usize])\n}\n\nfn main() {\n    let IntContainer = comptime Container(i32)\n    let FloatContainer = comptime Container(f64)\n}");
    }
}
