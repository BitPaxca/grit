#[cfg(test)]
mod tests {
    use crate::lexer::{Lexer, TokenKind};

    fn lex(src: &str) -> Vec<TokenKind> {
        let mut lexer = Lexer::new(src, "test.gr");
        let tokens = lexer.tokenize();
        assert!(!lexer.has_errors(), "Lexer errors: {:?}", lexer.error_strings());
        tokens.into_iter().map(|t| t.kind).collect()
    }

    fn lex_first(src: &str) -> TokenKind {
        lex(src).into_iter().next().unwrap()
    }

    // ── Keywords ─────────────────────────────────────────

    #[test]
    fn test_keywords() {
        assert_eq!(lex_first("fn"), TokenKind::Fn);
        assert_eq!(lex_first("let"), TokenKind::Let);
        assert_eq!(lex_first("var"), TokenKind::Var);
        assert_eq!(lex_first("struct"), TokenKind::Struct);
        assert_eq!(lex_first("enum"), TokenKind::Enum);
        assert_eq!(lex_first("trait"), TokenKind::Trait);
        assert_eq!(lex_first("impl"), TokenKind::Impl);
        assert_eq!(lex_first("import"), TokenKind::Import);
        assert_eq!(lex_first("return"), TokenKind::Return);
        assert_eq!(lex_first("if"), TokenKind::If);
        assert_eq!(lex_first("else"), TokenKind::Else);
        assert_eq!(lex_first("match"), TokenKind::Match);
        assert_eq!(lex_first("for"), TokenKind::For);
        assert_eq!(lex_first("while"), TokenKind::While);
        assert_eq!(lex_first("loop"), TokenKind::Loop);
        assert_eq!(lex_first("break"), TokenKind::Break);
        assert_eq!(lex_first("continue"), TokenKind::Continue);
        assert_eq!(lex_first("true"), TokenKind::True);
        assert_eq!(lex_first("false"), TokenKind::False);
        assert_eq!(lex_first("comptime"), TokenKind::Comptime);
        assert_eq!(lex_first("spawn"), TokenKind::Spawn);
        assert_eq!(lex_first("task"), TokenKind::Task);
        assert_eq!(lex_first("thread"), TokenKind::Thread);
        assert_eq!(lex_first("self"), TokenKind::SelfValue);
        assert_eq!(lex_first("pub"), TokenKind::Pub);
        assert_eq!(lex_first("const"), TokenKind::Const);
        assert_eq!(lex_first("type"), TokenKind::Type);
        assert_eq!(lex_first("defer"), TokenKind::Defer);
        assert_eq!(lex_first("where"), TokenKind::Where);
        assert_eq!(lex_first("owned"), TokenKind::Owned);
        assert_eq!(lex_first("raw"), TokenKind::Raw);
        assert_eq!(lex_first("trusted"), TokenKind::Trusted);
        assert_eq!(lex_first("dyn"), TokenKind::Dyn);
        assert_eq!(lex_first("extern"), TokenKind::Extern);
        assert_eq!(lex_first("and"), TokenKind::And);
        assert_eq!(lex_first("or"), TokenKind::Or);
        assert_eq!(lex_first("in"), TokenKind::In);
    }

    // ── Identifiers ──────────────────────────────────────

    #[test]
    fn test_identifiers() {
        assert_eq!(lex_first("hello"), TokenKind::Ident);
        assert_eq!(lex_first("_foo"), TokenKind::Ident);
        assert_eq!(lex_first("snake_case"), TokenKind::Ident);
        assert_eq!(lex_first("PascalCase"), TokenKind::Ident);
        assert_eq!(lex_first("x123"), TokenKind::Ident);
    }

    // ── Integer Literals ─────────────────────────────────

    #[test]
    fn test_decimal_integers() {
        assert_eq!(lex_first("0"), TokenKind::IntLiteral(0));
        assert_eq!(lex_first("42"), TokenKind::IntLiteral(42));
        assert_eq!(lex_first("1_000_000"), TokenKind::IntLiteral(1_000_000));
    }

    #[test]
    fn test_hex_integers() {
        assert_eq!(lex_first("0xFF"), TokenKind::IntLiteral(255));
        assert_eq!(lex_first("0xDEAD_BEEF"), TokenKind::IntLiteral(0xDEAD_BEEF));
    }

    #[test]
    fn test_octal_integers() {
        assert_eq!(lex_first("0o77"), TokenKind::IntLiteral(63));
        assert_eq!(lex_first("0o755"), TokenKind::IntLiteral(0o755));
    }

    #[test]
    fn test_binary_integers() {
        assert_eq!(lex_first("0b1010"), TokenKind::IntLiteral(0b1010));
        assert_eq!(lex_first("0b1111_0000"), TokenKind::IntLiteral(0b1111_0000));
    }

    // ── Float Literals ───────────────────────────────────

    #[test]
    fn test_floats() {
        assert_eq!(lex_first("3.14"), TokenKind::FloatLiteral(3.14));
        assert_eq!(lex_first("0.5"), TokenKind::FloatLiteral(0.5));
        assert_eq!(lex_first("1e10"), TokenKind::FloatLiteral(1e10));
        assert_eq!(lex_first("2.5e-3"), TokenKind::FloatLiteral(2.5e-3));
    }

    // ── String Literals ──────────────────────────────────

    #[test]
    fn test_strings() {
        assert_eq!(
            lex_first(r#""hello""#),
            TokenKind::StringLiteral("hello".to_string())
        );
        assert_eq!(
            lex_first(r#""hello\nworld""#),
            TokenKind::StringLiteral("hello\nworld".to_string())
        );
        assert_eq!(
            lex_first(r#""\t\r\n\\""#),
            TokenKind::StringLiteral("\t\r\n\\".to_string())
        );
    }

    #[test]
    fn test_unicode_escape() {
        assert_eq!(
            lex_first(r#""\u{1F600}""#),
            TokenKind::StringLiteral("\u{1F600}".to_string())
        );
    }

    // ── Char Literals ────────────────────────────────────

    #[test]
    fn test_chars() {
        assert_eq!(lex_first("'a'"), TokenKind::CharLiteral('a'));
        assert_eq!(lex_first("'\\n'"), TokenKind::CharLiteral('\n'));
        assert_eq!(lex_first("'\\0'"), TokenKind::CharLiteral('\0'));
    }

    // ── Operators ────────────────────────────────────────

    #[test]
    fn test_operators() {
        assert_eq!(lex_first("+"), TokenKind::Plus);
        assert_eq!(lex_first("-"), TokenKind::Minus);
        assert_eq!(lex_first("*"), TokenKind::Star);
        assert_eq!(lex_first("/"), TokenKind::Slash);
        assert_eq!(lex_first("%"), TokenKind::Percent);
        assert_eq!(lex_first("=="), TokenKind::EqEq);
        assert_eq!(lex_first("!="), TokenKind::BangEq);
        assert_eq!(lex_first("<="), TokenKind::LessEq);
        assert_eq!(lex_first(">="), TokenKind::GreaterEq);
        assert_eq!(lex_first("<<"), TokenKind::Shl);
        assert_eq!(lex_first(">>"), TokenKind::Shr);
        assert_eq!(lex_first("<<="), TokenKind::ShlEq);
        assert_eq!(lex_first(">>="), TokenKind::ShrEq);
        assert_eq!(lex_first("->"), TokenKind::Arrow);
        assert_eq!(lex_first("=>"), TokenKind::FatArrow);
        assert_eq!(lex_first("|>"), TokenKind::PipeArrow);
        assert_eq!(lex_first(".."), TokenKind::DotDot);
    }

    // ── Assignment Operators ─────────────────────────────

    #[test]
    fn test_assignment_ops() {
        assert_eq!(lex_first("="), TokenKind::Eq);
        assert_eq!(lex_first("+="), TokenKind::PlusEq);
        assert_eq!(lex_first("-="), TokenKind::MinusEq);
        assert_eq!(lex_first("*="), TokenKind::StarEq);
        assert_eq!(lex_first("/="), TokenKind::SlashEq);
        assert_eq!(lex_first("%="), TokenKind::PercentEq);
        assert_eq!(lex_first("&="), TokenKind::AmpEq);
        assert_eq!(lex_first("|="), TokenKind::PipeEq);
        assert_eq!(lex_first("^="), TokenKind::CaretEq);
    }

    // ── Comments ─────────────────────────────────────────

    #[test]
    fn test_line_comment() {
        let kinds = lex("42 // this is a comment\n");
        assert_eq!(kinds[0], TokenKind::IntLiteral(42));
        // comment is skipped, next meaningful token is Newline or Eof
    }

    #[test]
    fn test_block_comment() {
        let kinds = lex("42 /* block */ 7");
        assert_eq!(kinds[0], TokenKind::IntLiteral(42));
        assert_eq!(kinds[1], TokenKind::IntLiteral(7));
    }

    #[test]
    fn test_nested_block_comment() {
        let kinds = lex("42 /* outer /* inner */ still comment */ 7");
        assert_eq!(kinds[0], TokenKind::IntLiteral(42));
        assert_eq!(kinds[1], TokenKind::IntLiteral(7));
    }

    // ── Full program tokenization ────────────────────────

    #[test]
    fn test_hello_world() {
        let src = r#"fn main() {
    print("Hello, Grit.")
}"#;
        let kinds = lex(src);
        assert_eq!(kinds[0], TokenKind::Fn);
        assert_eq!(kinds[1], TokenKind::Ident); // main
        assert_eq!(kinds[2], TokenKind::LParen);
        assert_eq!(kinds[3], TokenKind::RParen);
        assert_eq!(kinds[4], TokenKind::LBrace);
        assert_eq!(kinds[5], TokenKind::Ident); // print
        assert_eq!(kinds[6], TokenKind::LParen);
        assert_eq!(kinds[7], TokenKind::StringLiteral("Hello, Grit.".to_string()));
        assert_eq!(kinds[8], TokenKind::RParen);
        assert_eq!(kinds[9], TokenKind::Newline);
        assert_eq!(kinds[10], TokenKind::RBrace);
    }

    #[test]
    fn test_let_var_binding() {
        let kinds = lex("let x = 42\nvar y = 10\n");
        assert_eq!(kinds[0], TokenKind::Let);
        assert_eq!(kinds[1], TokenKind::Ident);
        assert_eq!(kinds[2], TokenKind::Eq);
        assert_eq!(kinds[3], TokenKind::IntLiteral(42));
        assert_eq!(kinds[4], TokenKind::Newline);
        assert_eq!(kinds[5], TokenKind::Var);
        assert_eq!(kinds[6], TokenKind::Ident);
        assert_eq!(kinds[7], TokenKind::Eq);
        assert_eq!(kinds[8], TokenKind::IntLiteral(10));
        assert_eq!(kinds[9], TokenKind::Newline);
    }

    #[test]
    fn test_function_with_types() {
        let kinds = lex("fn add(a: i32, b: i32) -> i32 {\n    a + b\n}\n");
        assert_eq!(kinds[0], TokenKind::Fn);
        assert_eq!(kinds[1], TokenKind::Ident); // add
        assert_eq!(kinds[2], TokenKind::LParen);
        assert_eq!(kinds[3], TokenKind::Ident); // a
        assert_eq!(kinds[4], TokenKind::Colon);
        assert_eq!(kinds[5], TokenKind::Ident); // i32
        assert_eq!(kinds[6], TokenKind::Comma);
    }

    // ── Newline significance ─────────────────────────────

    #[test]
    fn test_newline_after_rparen() {
        let kinds = lex("foo()\n");
        assert!(kinds.contains(&TokenKind::Newline));
    }

    #[test]
    fn test_no_newline_after_operator() {
        // Newline after + should be ignored (not statement terminator)
        let kinds = lex("a +\nb");
        assert!(!kinds[0..kinds.len()-1].contains(&TokenKind::Newline));
    }
}
