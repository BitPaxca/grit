use super::token::{Token, TokenKind, Span};

/// The Grit lexer. Scans source text into a stream of tokens.
pub struct Lexer<'src> {
    source: &'src str,
    filename: String,
    bytes: &'src [u8],
    pos: usize,
    line: u32,
    col: u32,
    errors: Vec<String>,
    /// Track the last token kind for newline significance
    last_kind: Option<TokenKind>,
}

impl<'src> Lexer<'src> {
    pub fn new(source: &'src str, filename: &str) -> Self {
        Self {
            source,
            filename: filename.to_string(),
            bytes: source.as_bytes(),
            pos: 0,
            line: 1,
            col: 1,
            errors: Vec::new(),
            last_kind: None,
        }
    }

    pub fn has_errors(&self) -> bool {
        !self.errors.is_empty()
    }

    pub fn errors(&self) -> &[String] {
        &self.errors
    }

    /// Tokenize the entire source into a Vec of tokens.
    pub fn tokenize(&mut self) -> Vec<Token> {
        let mut tokens = Vec::new();
        loop {
            let tok = self.next_token();
            let is_eof = tok.kind == TokenKind::Eof;
            tokens.push(tok);
            if is_eof {
                break;
            }
        }
        tokens
    }

    // ── Core helpers ─────────────────────────────────────

    fn peek(&self) -> u8 {
        if self.pos < self.bytes.len() {
            self.bytes[self.pos]
        } else {
            0
        }
    }

    fn peek_next(&self) -> u8 {
        if self.pos + 1 < self.bytes.len() {
            self.bytes[self.pos + 1]
        } else {
            0
        }
    }

    fn advance(&mut self) -> u8 {
        let ch = self.peek();
        if ch == b'\n' {
            self.line += 1;
            self.col = 1;
        } else {
            self.col += 1;
        }
        self.pos += 1;
        ch
    }

    fn at_end(&self) -> bool {
        self.pos >= self.bytes.len()
    }

    fn lexeme(&self, start: usize) -> String {
        self.source[start..self.pos].to_string()
    }

    fn make_token(&mut self, kind: TokenKind, start: usize, start_line: u32, start_col: u32) -> Token {
        let span = Span::new(start, self.pos, start_line, start_col);
        let lexeme = self.lexeme(start);
        self.last_kind = Some(kind.clone());
        Token::new(kind, span, lexeme)
    }

    fn error_token(&mut self, msg: &str, start: usize, start_line: u32, start_col: u32) -> Token {
        let formatted = format!(
            "{}:{}:{}: error: {}",
            self.filename, start_line, start_col, msg
        );
        self.errors.push(formatted);
        self.make_token(TokenKind::Error(msg.to_string()), start, start_line, start_col)
    }

    /// Skip whitespace (spaces and tabs, NOT newlines — those are significant)
    fn skip_whitespace_and_comments(&mut self) {
        loop {
            match self.peek() {
                b' ' | b'\t' | b'\r' => {
                    self.advance();
                }
                b'/' if self.peek_next() == b'/' => {
                    // Line comment — skip to end of line
                    while !self.at_end() && self.peek() != b'\n' {
                        self.advance();
                    }
                }
                b'/' if self.peek_next() == b'*' => {
                    // Block comment — handle nesting
                    self.advance(); // /
                    self.advance(); // *
                    let mut depth = 1u32;
                    while !self.at_end() && depth > 0 {
                        if self.peek() == b'/' && self.peek_next() == b'*' {
                            self.advance();
                            self.advance();
                            depth += 1;
                        } else if self.peek() == b'*' && self.peek_next() == b'/' {
                            self.advance();
                            self.advance();
                            depth -= 1;
                        } else {
                            self.advance();
                        }
                    }
                }
                _ => break,
            }
        }
    }

    // ── Main dispatch ────────────────────────────────────

    pub fn next_token(&mut self) -> Token {
        self.skip_whitespace_and_comments();

        if self.at_end() {
            return self.make_token(TokenKind::Eof, self.pos, self.line, self.col);
        }

        let start = self.pos;
        let start_line = self.line;
        let start_col = self.col;
        let ch = self.advance();

        match ch {
            // Newlines — significant if the previous token could end a statement
            b'\n' => {
                if self.last_kind.as_ref().map_or(false, |k| k.ends_statement()) {
                    self.make_token(TokenKind::Newline, start, start_line, start_col)
                } else {
                    // Insignificant newline — skip and get next token
                    self.next_token()
                }
            }

            // Single-char delimiters
            b'(' => self.make_token(TokenKind::LParen, start, start_line, start_col),
            b')' => self.make_token(TokenKind::RParen, start, start_line, start_col),
            b'{' => self.make_token(TokenKind::LBrace, start, start_line, start_col),
            b'}' => self.make_token(TokenKind::RBrace, start, start_line, start_col),
            b'[' => self.make_token(TokenKind::LBracket, start, start_line, start_col),
            b']' => self.make_token(TokenKind::RBracket, start, start_line, start_col),
            b',' => self.make_token(TokenKind::Comma, start, start_line, start_col),
            b';' => self.make_token(TokenKind::Newline, start, start_line, start_col),
            b'~' => self.make_token(TokenKind::Tilde, start, start_line, start_col),
            b'#' => self.make_token(TokenKind::Hash, start, start_line, start_col),

            // Dot or DotDot
            b'.' if self.peek() == b'.' => {
                self.advance();
                self.make_token(TokenKind::DotDot, start, start_line, start_col)
            }
            b'.' => self.make_token(TokenKind::Dot, start, start_line, start_col),

            // Colon
            b':' => self.make_token(TokenKind::Colon, start, start_line, start_col),

            // Operators with = variants
            b'+' if self.peek() == b'=' => {
                self.advance();
                self.make_token(TokenKind::PlusEq, start, start_line, start_col)
            }
            b'+' => self.make_token(TokenKind::Plus, start, start_line, start_col),

            b'-' if self.peek() == b'>' => {
                self.advance();
                self.make_token(TokenKind::Arrow, start, start_line, start_col)
            }
            b'-' if self.peek() == b'=' => {
                self.advance();
                self.make_token(TokenKind::MinusEq, start, start_line, start_col)
            }
            b'-' => self.make_token(TokenKind::Minus, start, start_line, start_col),

            b'*' if self.peek() == b'=' => {
                self.advance();
                self.make_token(TokenKind::StarEq, start, start_line, start_col)
            }
            b'*' => self.make_token(TokenKind::Star, start, start_line, start_col),

            b'/' if self.peek() == b'=' => {
                self.advance();
                self.make_token(TokenKind::SlashEq, start, start_line, start_col)
            }
            b'/' => self.make_token(TokenKind::Slash, start, start_line, start_col),

            b'%' if self.peek() == b'=' => {
                self.advance();
                self.make_token(TokenKind::PercentEq, start, start_line, start_col)
            }
            b'%' => self.make_token(TokenKind::Percent, start, start_line, start_col),

            b'&' if self.peek() == b'=' => {
                self.advance();
                self.make_token(TokenKind::AmpEq, start, start_line, start_col)
            }
            b'&' => self.make_token(TokenKind::Ampersand, start, start_line, start_col),

            b'|' if self.peek() == b'>' => {
                self.advance();
                self.make_token(TokenKind::PipeArrow, start, start_line, start_col)
            }
            b'|' if self.peek() == b'=' => {
                self.advance();
                self.make_token(TokenKind::PipeEq, start, start_line, start_col)
            }
            b'|' => self.make_token(TokenKind::Pipe, start, start_line, start_col),

            b'^' if self.peek() == b'=' => {
                self.advance();
                self.make_token(TokenKind::CaretEq, start, start_line, start_col)
            }
            b'^' => self.make_token(TokenKind::Caret, start, start_line, start_col),

            b'!' if self.peek() == b'=' => {
                self.advance();
                self.make_token(TokenKind::BangEq, start, start_line, start_col)
            }
            b'!' => self.make_token(TokenKind::Bang, start, start_line, start_col),

            b'=' if self.peek() == b'=' => {
                self.advance();
                self.make_token(TokenKind::EqEq, start, start_line, start_col)
            }
            b'=' if self.peek() == b'>' => {
                self.advance();
                self.make_token(TokenKind::FatArrow, start, start_line, start_col)
            }
            b'=' => self.make_token(TokenKind::Eq, start, start_line, start_col),

            b'<' if self.peek() == b'<' && self.peek_next() == b'=' => {
                self.advance(); self.advance();
                self.make_token(TokenKind::ShlEq, start, start_line, start_col)
            }
            b'<' if self.peek() == b'<' => {
                self.advance();
                self.make_token(TokenKind::Shl, start, start_line, start_col)
            }
            b'<' if self.peek() == b'=' => {
                self.advance();
                self.make_token(TokenKind::LessEq, start, start_line, start_col)
            }
            b'<' => self.make_token(TokenKind::Less, start, start_line, start_col),

            b'>' if self.peek() == b'>' && self.peek_next() == b'=' => {
                self.advance(); self.advance();
                self.make_token(TokenKind::ShrEq, start, start_line, start_col)
            }
            b'>' if self.peek() == b'>' => {
                self.advance();
                self.make_token(TokenKind::Shr, start, start_line, start_col)
            }
            b'>' if self.peek() == b'=' => {
                self.advance();
                self.make_token(TokenKind::GreaterEq, start, start_line, start_col)
            }
            b'>' => self.make_token(TokenKind::Greater, start, start_line, start_col),

            b'?' => self.make_token(TokenKind::Question, start, start_line, start_col),

            // String literals
            b'"' => self.scan_string(start, start_line, start_col),

            // Byte string literals
            b'b' if self.peek() == b'"' => {
                self.advance(); // consume the opening "
                self.scan_byte_string(start, start_line, start_col)
            }

            // Raw string literals
            b'r' if self.peek() == b'"' => {
                self.advance();
                self.scan_raw_string(start, start_line, start_col)
            }

            // Char literals
            b'\'' => self.scan_char(start, start_line, start_col),

            // Number literals
            b'0'..=b'9' => self.scan_number(ch, start, start_line, start_col),

            // Identifiers and keywords
            b'a'..=b'z' | b'A'..=b'Z' | b'_' => {
                self.scan_ident(start, start_line, start_col)
            }

            _ => self.error_token(
                &format!("unexpected character '{}'", ch as char),
                start, start_line, start_col,
            ),
        }
    }

    // ── Identifier & keyword scanner ─────────────────────

    fn scan_ident(&mut self, start: usize, start_line: u32, start_col: u32) -> Token {
        while !self.at_end() && (self.peek().is_ascii_alphanumeric() || self.peek() == b'_') {
            self.advance();
        }
        let text = &self.source[start..self.pos];

        // Check for 'b"' byte string prefix handled earlier in dispatch,
        // but 'b' alone is just an identifier
        let kind = TokenKind::keyword(text).unwrap_or(TokenKind::Ident);
        self.make_token(kind, start, start_line, start_col)
    }

    // ── Number scanner ───────────────────────────────────

    fn scan_number(&mut self, first: u8, start: usize, start_line: u32, start_col: u32) -> Token {
        // Check for hex, octal, binary prefixes
        if first == b'0' && !self.at_end() {
            match self.peek() {
                b'x' | b'X' => return self.scan_hex(start, start_line, start_col),
                b'o' | b'O' => return self.scan_oct(start, start_line, start_col),
                b'b' if self.peek_next() != b'"' => {
                    // 0b but not 0b" (which would be weird anyway)
                    if self.peek_next() == b'0' || self.peek_next() == b'1' || self.peek_next() == b'_' {
                        return self.scan_bin(start, start_line, start_col);
                    }
                }
                _ => {}
            }
        }

        // Decimal integer or float
        self.eat_digits();

        // Check for float: decimal point followed by digit
        if self.peek() == b'.' && self.peek_next().is_ascii_digit() {
            self.advance(); // consume '.'
            self.eat_digits();

            // Exponent
            if self.peek() == b'e' || self.peek() == b'E' {
                self.advance();
                if self.peek() == b'+' || self.peek() == b'-' {
                    self.advance();
                }
                self.eat_digits();
            }

            // Float suffix
            self.eat_float_suffix();

            let text = self.source[start..self.pos].replace('_', "");
            match text.trim_end_matches(|c: char| c.is_alphabetic()).parse::<f64>() {
                Ok(val) => self.make_token(TokenKind::FloatLiteral(val), start, start_line, start_col),
                Err(_) => self.error_token("invalid float literal", start, start_line, start_col),
            }
        } else {
            // Check for exponent without decimal (e.g., 1e10)
            if self.peek() == b'e' || self.peek() == b'E' {
                self.advance();
                if self.peek() == b'+' || self.peek() == b'-' {
                    self.advance();
                }
                self.eat_digits();
                self.eat_float_suffix();
                let text = self.source[start..self.pos].replace('_', "");
                match text.trim_end_matches(|c: char| c.is_alphabetic()).parse::<f64>() {
                    Ok(val) => self.make_token(TokenKind::FloatLiteral(val), start, start_line, start_col),
                    Err(_) => self.error_token("invalid float literal", start, start_line, start_col),
                }
            } else {
                // Integer with optional suffix
                self.eat_int_suffix();
                let text = self.source[start..self.pos].replace('_', "");
                let clean = text.trim_end_matches(|c: char| c.is_alphabetic());
                match clean.parse::<u128>() {
                    Ok(val) => self.make_token(TokenKind::IntLiteral(val), start, start_line, start_col),
                    Err(_) => self.error_token("integer literal too large", start, start_line, start_col),
                }
            }
        }
    }

    fn scan_hex(&mut self, start: usize, start_line: u32, start_col: u32) -> Token {
        self.advance(); // skip 'x'
        if !self.peek().is_ascii_hexdigit() {
            return self.error_token("expected hex digit after 0x", start, start_line, start_col);
        }
        while !self.at_end() && (self.peek().is_ascii_hexdigit() || self.peek() == b'_') {
            self.advance();
        }
        let suffix_start = self.pos;
        self.eat_int_suffix();
        let hex_end = suffix_start;  // digits end where suffix begins
        let text = self.source[start+2..hex_end].replace('_', "");
        match u128::from_str_radix(&text, 16) {
            Ok(val) => self.make_token(TokenKind::IntLiteral(val), start, start_line, start_col),
            Err(_) => self.error_token("invalid hex literal", start, start_line, start_col),
        }
    }

    fn scan_oct(&mut self, start: usize, start_line: u32, start_col: u32) -> Token {
        self.advance(); // skip 'o'
        if !(self.peek() >= b'0' && self.peek() <= b'7') {
            return self.error_token("expected octal digit after 0o", start, start_line, start_col);
        }
        while !self.at_end() && ((self.peek() >= b'0' && self.peek() <= b'7') || self.peek() == b'_') {
            self.advance();
        }
        self.eat_int_suffix();
        let text = self.source[start+2..self.pos].replace('_', "");
        let clean = text.trim_end_matches(|c: char| c.is_alphabetic());
        match u128::from_str_radix(clean, 8) {
            Ok(val) => self.make_token(TokenKind::IntLiteral(val), start, start_line, start_col),
            Err(_) => self.error_token("invalid octal literal", start, start_line, start_col),
        }
    }

    fn scan_bin(&mut self, start: usize, start_line: u32, start_col: u32) -> Token {
        self.advance(); // skip 'b'
        if self.peek() != b'0' && self.peek() != b'1' {
            return self.error_token("expected binary digit after 0b", start, start_line, start_col);
        }
        while !self.at_end() && (self.peek() == b'0' || self.peek() == b'1' || self.peek() == b'_') {
            self.advance();
        }
        self.eat_int_suffix();
        let text = self.source[start+2..self.pos].replace('_', "");
        let clean = text.trim_end_matches(|c: char| c.is_alphabetic());
        match u128::from_str_radix(clean, 2) {
            Ok(val) => self.make_token(TokenKind::IntLiteral(val), start, start_line, start_col),
            Err(_) => self.error_token("invalid binary literal", start, start_line, start_col),
        }
    }

    fn eat_digits(&mut self) {
        while !self.at_end() && (self.peek().is_ascii_digit() || self.peek() == b'_') {
            self.advance();
        }
    }

    fn eat_int_suffix(&mut self) {
        let remaining = &self.source[self.pos..];
        let suffixes = [
            "usize", "isize", "u128", "i128", "u64", "i64",
            "u32", "i32", "u16", "i16", "u8", "i8",
        ];
        for s in &suffixes {
            if remaining.starts_with(s) {
                for _ in 0..s.len() {
                    self.advance();
                }
                return;
            }
        }
    }

    fn eat_float_suffix(&mut self) {
        let remaining = &self.source[self.pos..];
        if remaining.starts_with("f64") || remaining.starts_with("f32") {
            for _ in 0..3 {
                self.advance();
            }
        }
    }

    // ── String scanner ───────────────────────────────────

    fn scan_string(&mut self, start: usize, start_line: u32, start_col: u32) -> Token {
        let mut value = String::new();
        while !self.at_end() && self.peek() != b'"' {
            if self.peek() == b'\n' {
                return self.error_token("unterminated string literal", start, start_line, start_col);
            }
            if self.peek() == b'\\' {
                self.advance();
                match self.scan_escape_char() {
                    Ok(c) => value.push(c),
                    Err(msg) => return self.error_token(&msg, start, start_line, start_col),
                }
            } else {
                value.push(self.advance() as char);
            }
        }
        if self.at_end() {
            return self.error_token("unterminated string literal", start, start_line, start_col);
        }
        self.advance(); // closing "
        self.make_token(TokenKind::StringLiteral(value), start, start_line, start_col)
    }

    fn scan_raw_string(&mut self, start: usize, start_line: u32, start_col: u32) -> Token {
        let mut value = String::new();
        while !self.at_end() && self.peek() != b'"' {
            value.push(self.advance() as char);
        }
        if self.at_end() {
            return self.error_token("unterminated raw string literal", start, start_line, start_col);
        }
        self.advance(); // closing "
        self.make_token(TokenKind::StringLiteral(value), start, start_line, start_col)
    }

    fn scan_byte_string(&mut self, start: usize, start_line: u32, start_col: u32) -> Token {
        let mut value = Vec::new();
        while !self.at_end() && self.peek() != b'"' {
            if self.peek() == b'\n' {
                return self.error_token("unterminated byte string literal", start, start_line, start_col);
            }
            if self.peek() == b'\\' {
                self.advance();
                match self.scan_escape_char() {
                    Ok(c) => value.push(c as u8),
                    Err(msg) => return self.error_token(&msg, start, start_line, start_col),
                }
            } else {
                value.push(self.advance());
            }
        }
        if self.at_end() {
            return self.error_token("unterminated byte string literal", start, start_line, start_col);
        }
        self.advance(); // closing "
        self.make_token(TokenKind::ByteStringLiteral(value), start, start_line, start_col)
    }

    // ── Char scanner ─────────────────────────────────────

    fn scan_char(&mut self, start: usize, start_line: u32, start_col: u32) -> Token {
        if self.at_end() {
            return self.error_token("unterminated character literal", start, start_line, start_col);
        }
        let ch = if self.peek() == b'\\' {
            self.advance();
            match self.scan_escape_char() {
                Ok(c) => c,
                Err(msg) => return self.error_token(&msg, start, start_line, start_col),
            }
        } else {
            self.advance() as char
        };

        if self.at_end() || self.peek() != b'\'' {
            return self.error_token("unterminated character literal — expected closing '", start, start_line, start_col);
        }
        self.advance(); // closing '
        self.make_token(TokenKind::CharLiteral(ch), start, start_line, start_col)
    }

    // ── Escape sequences ─────────────────────────────────

    fn scan_escape_char(&mut self) -> Result<char, String> {
        if self.at_end() {
            return Err("unexpected end of file in escape sequence".to_string());
        }
        let ch = self.advance();
        match ch {
            b'n' => Ok('\n'),
            b'r' => Ok('\r'),
            b't' => Ok('\t'),
            b'\\' => Ok('\\'),
            b'\'' => Ok('\''),
            b'"' => Ok('"'),
            b'0' => Ok('\0'),
            b'x' => {
                let hi = self.advance();
                let lo = self.advance();
                let hex = format!("{}{}", hi as char, lo as char);
                match u8::from_str_radix(&hex, 16) {
                    Ok(val) => Ok(val as char),
                    Err(_) => Err(format!("invalid hex escape \\x{}", hex)),
                }
            }
            b'u' => {
                if self.peek() != b'{' {
                    return Err("expected '{' after \\u".to_string());
                }
                self.advance(); // {
                let hex_start = self.pos;
                while !self.at_end() && self.peek() != b'}' {
                    self.advance();
                }
                if self.at_end() {
                    return Err("unterminated unicode escape".to_string());
                }
                let hex = &self.source[hex_start..self.pos];
                self.advance(); // }
                match u32::from_str_radix(hex, 16) {
                    Ok(val) => match char::from_u32(val) {
                        Some(c) => Ok(c),
                        None => Err(format!("invalid unicode codepoint: U+{:04X}", val)),
                    },
                    Err(_) => Err(format!("invalid unicode escape \\u{{{}}}", hex)),
                }
            }
            _ => Err(format!("unknown escape sequence '\\{}'", ch as char)),
        }
    }
}
