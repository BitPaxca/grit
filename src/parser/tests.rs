#[cfg(test)]
mod tests {
    use crate::lexer::Lexer;
    use crate::parser::Parser;
    use crate::ast::*;

    fn parse(src: &str) -> SourceFile {
        let mut lexer = Lexer::new(src, "test.gr");
        let tokens = lexer.tokenize();
        assert!(!lexer.has_errors(), "Lex errors: {:?}", lexer.error_strings());
        let mut parser = Parser::new(tokens);
        let program = parser.parse_program();
        if parser.has_errors() {
            panic!("Parse errors: {:?}", parser.error_strings());
        }
        program
    }

    #[test]
    fn test_empty_program() {
        let p = parse("");
        assert_eq!(p.items.len(), 0);
    }

    #[test]
    fn test_hello_world() {
        let p = parse(r#"fn main() {
    print("Hello, Grit.")
}"#);
        assert_eq!(p.items.len(), 1);
        match &p.items[0] {
            Item::Function(f) => {
                assert_eq!(f.name, "main");
                assert_eq!(f.params.len(), 0);
                assert!(f.body.is_some());
            }
            _ => panic!("expected function"),
        }
    }

    #[test]
    fn test_function_with_params() {
        let p = parse("fn add(a: i32, b: i32) -> i32 {\n    a + b\n}");
        match &p.items[0] {
            Item::Function(f) => {
                assert_eq!(f.name, "add");
                assert_eq!(f.params.len(), 2);
                assert_eq!(f.params[0].name, "a");
                assert_eq!(f.params[1].name, "b");
                assert!(f.return_type.is_some());
            }
            _ => panic!("expected function"),
        }
    }

    #[test]
    fn test_struct() {
        let p = parse("struct Point {\n    x: f64\n    y: f64\n}");
        match &p.items[0] {
            Item::Struct(s) => {
                assert_eq!(s.name, "Point");
                assert_eq!(s.fields.len(), 2);
                assert_eq!(s.fields[0].name, "x");
                assert_eq!(s.fields[1].name, "y");
            }
            _ => panic!("expected struct"),
        }
    }

    #[test]
    fn test_enum() {
        let p = parse("enum Color {\n    Red\n    Green\n    Blue\n}");
        match &p.items[0] {
            Item::Enum(e) => {
                assert_eq!(e.name, "Color");
                assert_eq!(e.variants.len(), 3);
            }
            _ => panic!("expected enum"),
        }
    }

    #[test]
    fn test_enum_with_data() {
        let p = parse("enum Shape {\n    Circle { radius: f64 }\n    Rect(f64, f64)\n}");
        match &p.items[0] {
            Item::Enum(e) => {
                assert_eq!(e.variants.len(), 2);
                assert!(matches!(e.variants[0].fields, VariantKind::Struct(_)));
                assert!(matches!(e.variants[1].fields, VariantKind::Tuple(_)));
            }
            _ => panic!("expected enum"),
        }
    }

    #[test]
    fn test_trait() {
        let p = parse("trait Drawable {\n    fn draw(self)\n}");
        match &p.items[0] {
            Item::Trait(t) => {
                assert_eq!(t.name, "Drawable");
                assert_eq!(t.methods.len(), 1);
            }
            _ => panic!("expected trait"),
        }
    }

    #[test]
    fn test_impl_block() {
        let p = parse("impl Point {\n    fn new(x: f64, y: f64) -> Point {\n        x\n    }\n}");
        match &p.items[0] {
            Item::Impl(i) => {
                assert_eq!(i.target, "Point");
                assert!(i.trait_name.is_none());
                assert_eq!(i.methods.len(), 1);
            }
            _ => panic!("expected impl"),
        }
    }

    #[test]
    fn test_impl_trait_for() {
        let p = parse("impl Drawable for Circle {\n    fn draw(self) {\n        self\n    }\n}");
        match &p.items[0] {
            Item::Impl(i) => {
                assert_eq!(i.target, "Circle");
                assert_eq!(i.trait_name.as_deref(), Some("Drawable"));
            }
            _ => panic!("expected impl"),
        }
    }

    #[test]
    fn test_import() {
        let p = parse("import std.io.File\n");
        match &p.items[0] {
            Item::Import(i) => {
                assert_eq!(i.path, vec!["std", "io", "File"]);
                assert!(i.names.is_none());
            }
            _ => panic!("expected import"),
        }
    }

    #[test]
    fn test_import_multi() {
        let p = parse("import std.io.{File, stdin}\n");
        match &p.items[0] {
            Item::Import(i) => {
                assert_eq!(i.path, vec!["std", "io"]);
                assert_eq!(i.names.as_ref().unwrap(), &vec!["File", "stdin"]);
            }
            _ => panic!("expected import"),
        }
    }

    #[test]
    fn test_let_binding() {
        let p = parse("fn main() {\n    let x = 42\n}");
        match &p.items[0] {
            Item::Function(f) => {
                let block = f.body.as_ref().unwrap();
                assert_eq!(block.stmts.len(), 1);
                assert!(matches!(&block.stmts[0], Stmt::Let(_)));
            }
            _ => panic!("expected function"),
        }
    }

    #[test]
    fn test_var_binding() {
        let p = parse("fn main() {\n    var x = 10\n}");
        match &p.items[0] {
            Item::Function(f) => {
                let block = f.body.as_ref().unwrap();
                assert!(matches!(&block.stmts[0], Stmt::Var(_)));
            }
            _ => panic!("expected function"),
        }
    }

    #[test]
    fn test_binary_expr() {
        let p = parse("fn main() {\n    let x = 1 + 2 * 3\n}");
        match &p.items[0] {
            Item::Function(f) => {
                let block = f.body.as_ref().unwrap();
                if let Stmt::Let(l) = &block.stmts[0] {
                    // Should be Add(1, Mul(2, 3)) due to precedence
                    assert!(matches!(&l.value, Expr::Binary { op: BinOp::Add, .. }));
                }
            }
            _ => panic!("expected function"),
        }
    }

    #[test]
    fn test_if_expr() {
        let p = parse("fn main() {\n    if x > 0 {\n        x\n    } else {\n        y\n    }\n}");
        assert_eq!(p.items.len(), 1);
    }

    #[test]
    fn test_method_call() {
        let p = parse("fn main() {\n    buf.write(data)\n}");
        assert_eq!(p.items.len(), 1);
    }

    #[test]
    fn test_self_param() {
        let p = parse("fn get(self) -> i32 {\n    42\n}");
        match &p.items[0] {
            Item::Function(f) => {
                assert!(f.params[0].is_self);
                assert_eq!(f.params[0].name, "self");
            }
            _ => panic!("expected function"),
        }
    }

    #[test]
    fn test_var_self_param() {
        let p = parse("fn set(var self, x: i32) {\n    42\n}");
        match &p.items[0] {
            Item::Function(f) => {
                assert!(f.params[0].is_self);
                assert!(f.params[0].is_var);
            }
            _ => panic!("expected function"),
        }
    }
}
