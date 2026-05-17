#[cfg(test)]
mod tests {
    use crate::lexer::Lexer;
    use crate::parser::Parser;
    use crate::typeck::TypeChecker;

    fn check(src: &str) -> Vec<String> {
        let mut lexer = Lexer::new(src, "test.gr");
        let tokens = lexer.tokenize();
        assert!(!lexer.has_errors(), "Lex errors: {:?}", lexer.errors());
        let mut parser = Parser::new(tokens);
        let program = parser.parse_program();
        assert!(!parser.has_errors(), "Parse errors: {:?}", parser.errors());
        let mut checker = TypeChecker::new();
        checker.check_program(&program);
        checker.errors().to_vec()
    }

    fn check_ok(src: &str) {
        let errors = check(src);
        assert!(errors.is_empty(), "Unexpected type errors: {:?}", errors);
    }

    fn check_err(src: &str) -> Vec<String> {
        let errors = check(src);
        assert!(!errors.is_empty(), "Expected type errors but got none");
        errors
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
}
