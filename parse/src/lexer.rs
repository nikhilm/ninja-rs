use std::slice::Iter;

/// Reflects a position in the stream. This can be translated to a line+column Position using
/// Lexer::to_position.
pub struct Pos(usize); // This way, it is only possible to obtain a Pos from a token/error.

#[derive(Debug, PartialEq, Eq)]
pub struct Position {
    filename: Option<String>, // TODO: &str; also, comparing Eq using filenames does not make sense.
    line: usize,
    column: usize,
}

impl Position {
    fn new(filename: Option<String>, line: usize, column: usize) -> Position {
        Position {
            filename: filename,
            line: line,
            column: column,
        }
    }

    fn untitled(line: usize, column: usize) -> Position {
        Position {
            filename: None,
            line: line,
            column: column,
        }
    }
}

#[derive(Debug, PartialEq, Eq)]
pub enum Token<'a> {
    Build,
    Colon,
    Default,
    Escape,
    Equals,
    Identifier(&'a [u8]),
    Illegal(u8),
    Comment(&'a [u8]),
    Include,
    Indent,
    Newline,
    Pipe,
    Pipe2,
    Pool,
    Rule,
    Subninja,
}

type TokenPosition<'a> = (Token<'a>, Position);

type ErrorHandler<'e> = Box<dyn FnMut(Position, &str) + 'e>;

pub struct Lexer<'a, 'b> {
    data: &'a [u8],
    filename: Option<String>,
    ch: u8,
    done: bool,
    offset: usize,
    next_offset: usize,
    // consider using `smallvec` later.
    line_offsets: Vec<usize>,
    error_handler: Option<ErrorHandler<'b>>,
    pub error_count: u32,
}

impl<'a, 'b> Lexer<'a, 'b> {
    pub fn new(
        data: &'a [u8],
        filename: Option<String>,
        handler: Option<ErrorHandler<'b>>,
    ) -> Lexer<'a, 'b> {
        Lexer {
            data: data,
            filename: filename,
            // This allows skip_horizontal_whitespace as the first call to advance by one and set
            // everything up.
            ch: ' ' as u8,
            done: false,
            offset: 0,
            next_offset: 0,
            line_offsets: vec![0],
            error_handler: handler,
            error_count: 0,
        }
    }

    /*

    /// Also skips newlines. Be careful!
    fn skip_whitespace(&mut self) {
        while let Some(c) = self.peek_byte() {
            if c.is_ascii_whitespace() {
                self.read_byte();
            } else {
                break;
            }
        }
    }
    */

    fn error(&mut self, pos: usize, reason: &str) {
        if self.error_handler.is_some() {
            let pos = self.to_position(Pos(pos));
            self.error_handler.as_mut().unwrap()(pos, reason);
        }
        self.error_count += 1;
    }

    fn skip_horizontal_whitespace(&mut self) {
        while self.ch == (' ' as u8) || self.ch == ('\t' as u8) {
            self.advance();
        }
    }

    fn read_identifier(&mut self) -> Token<'a> {
        // The Ninja manual doesn't really define what an identifier is. Since we need to handle
        // paths, we keep going until whitespace.

        let span_start = self.offset;
        let mut span_end = self.offset; // exclusive
        while !self.ch.is_ascii_whitespace() {
            span_end += 1;
            if self.advance().is_none() {
                break;
            }
        }
        Token::Identifier(&self.data[span_start..span_end])
    }

    fn lookup_keyword(ident: Token) -> Token {
        match ident {
            Token::Identifier(slice) => match slice {
                // Know a better way than this? as_bytes() is not allowed here.
                [98, 117, 105, 108, 100] => Token::Build,
                [100, 101, 102, 97, 117, 108, 116] => Token::Default,
                [105, 110, 99, 108, 117, 100, 101] => Token::Include,
                [112, 111, 111, 108] => Token::Pool,
                [114, 117, 108, 101] => Token::Rule,
                [115, 117, 98, 110, 105, 110, 106, 97] => Token::Subninja,
                _ => ident,
            },
            _ => {
                panic!("Expected identifier");
            }
        }
    }

    fn record_line(&mut self) {
        self.line_offsets.push(self.offset);
    }

    fn advance(&mut self) -> Option<u8> {
        // This exists to make sure we do not set next_offset to 1 on the very first read.
        if self.next_offset < self.data.len() {
            self.offset = self.next_offset;
            self.ch = self.data[self.next_offset];
            self.next_offset += 1;
            Some(self.ch)
        } else {
            self.offset = self.data.len();
            // TODO: Make self.ch unrepresentable.
            self.ch = 0;
            None
        }
    }

    fn done(&self) -> bool {
        self.offset >= self.data.len()
    }

    fn to_position(&self, pos: Pos) -> Position {
        // maybe a consumed Lexer _should_ return some new object? that has line offsets and error
        // things populated?
        assert!(self.done());
        assert!(self.line_offsets.is_sorted());
        if pos.0 >= self.data.len() {
            panic!("position past end of data");
        }

        match self.line_offsets.binary_search(&pos.0) {
            Ok(idx) => Position::new(self.filename.clone(), idx + 1, 1),
            Err(idx) => {
                // Since 0 is the first element in the vec, nothing can be inserted before that, at
                // position 0.
                assert!(idx > 0);
                Position::new(
                    self.filename.clone(),
                    idx,
                    pos.0 - self.line_offsets[idx - 1] + 1,
                )
            }
        }
    }

    fn read_comment(&mut self) -> Token<'a> {
        // TODO: Handle \r\n
        let start = self.offset - 1; // Includes the '#' in the comment.
        let mut end = self.offset;
        while !self.done() && self.ch != ('\n' as u8) {
            end += 1;
            self.advance();
        }
        Token::Comment(&self.data[start..end])
    }
    /*
    fn peek(&mut self) -> Option<u8> {
        assert!(
            (self.next_offset == self.offset + 1) || (self.next_offset == 0 && self.offset == 0)
        );
        if self.next_offset < self.data.len() {
            Some(self.data[self.next_offset])
        } else {
            None
        }
    }
    */
}

impl<'a, 'b> Iterator for Lexer<'a, 'b> {
    type Item = Token<'a>;
    // A ninja file lexer should not evaluate variables. It should only emit a token stream. This
    // means things like subninja/include do not affect the lexer, they are just keywords. On the
    // other hand, leading whitespace is significant, and does affect the lexer. In addition, `$`
    // affects how things are interpreted at the lexer stage, as it means newlines need to be
    // preserved. In addition, something like
    // ```
    // foo = bar $
    //      baz
    // ```
    // Does not start a new scope. The lexer should not be aware of scopes, but $ should trigger
    // special processing. That means leading whitespace should be preserved and emitted as a
    // token, with some awareness of "how much" whitespace there is.
    // In addition, need to determine how to capture error locations for good reporting.
    // Since escapes and so on should not be processed in comments, the lexer needs to be aware of
    // that.
    // Beginning of line whitespace handling:
    //   If the previous line is continuing, this is discarded, otherwise it has meaning.
    // Should the lexer simply emit the token stream, i.e. DOLLAR, NEWLINE,
    // WHITESPACE(<actual>)...?
    fn next(&mut self) -> Option<Self::Item> {
        self.skip_horizontal_whitespace();
        if self.done() {
            return None;
        }

        if !self.ch.is_ascii() {
            // We are in the top-level loop and got a non-ASCII character. No idea how to
            // handle this!
            // TODO: Report useful error.
            let err = format!("Unexpected byte: {}", self.ch);
            self.error(self.offset, &err);
            self.advance();
            return Some(Token::Illegal(self.ch));
        }

        if self.ch.is_ascii_alphabetic() {
            Some(Lexer::lookup_keyword(self.read_identifier()))
        } else {
            let ch = self.ch;
            // Always make progress.
            let next = self.advance();
            match ch as char {
                // TODO: Windows line ending support.
                // Also not sure if yielding a newline token in the general case really makes
                // sense. Ninja is sensitive about that only in certain cases.
                '\n' => {
                    self.record_line();
                    Some(Token::Newline)
                }
                '\t' => Some(Token::Illegal('\t' as u8)),
                // TODO: Handle indentation.
                '=' => Some(Token::Equals),
                ':' => Some(Token::Colon),
                '|' => {
                    if let Some(c) = next {
                        if c == ('|' as u8) {
                            self.advance();
                            Some(Token::Pipe2)
                        } else {
                            Some(Token::Pipe)
                        }
                    } else {
                        Some(Token::Pipe)
                    }
                }
                '$' => Some(Token::Escape),
                '#' => Some(self.read_comment()),
                _ => {
                    let err = format!("Unexpected character: {}", ch as char);
                    self.error(self.offset - 1, &err);
                    Some(Token::Illegal(ch))
                }
            }
        }
    }
}

#[cfg(test)]
mod test {
    use super::Lexer;
    use super::Pos;
    use super::Position;
    use super::Token;
    // This may be a good place to use the `insta` crate, but possibly overkill as well.

    fn parse_and_slice(input: &str) -> Vec<Token> {
        let lexer = Lexer::new(input.as_bytes(), None, None);
        lexer.collect::<Vec<Token>>()
    }

    #[test]
    fn test_simple() {
        assert_eq!(&parse_and_slice(":"), &[Token::Colon]);
    }

    #[test]
    fn test_pool_simple() {
        let stream = parse_and_slice("pool chairs");
        assert_eq!(stream[0], Token::Pool);
        match stream[1] {
            Token::Identifier(span) => {
                assert_eq!(span, "chairs".as_bytes());
            }
            _ => panic!("Unexpected token {:?}", stream[1]),
        };
    }

    #[test]
    fn test_error_triggered() {
        // This interface is not very ergonomic...
        let mut lexer = Lexer::new("pool )".as_bytes(), None, None);
        for token in &mut lexer {}
        assert_eq!(lexer.error_count, 1);
    }

    #[test]
    fn test_error_handler() {
        let mut handler_called = 0;
        {
            let handler = |pos: Position, err: &str| {
                // Now this would need a ref to the lexer again to translate the pos to a Position.
                // Which, again, needs a better interface.
                // fn error() already borrows as mutable, so it can't pass a reference here.
                handler_called += 1;
            };

            // This interface is not very ergonomic...
            let mut lexer = Lexer::new("pool )".as_bytes(), None, Some(Box::new(handler)));
            for token in &mut lexer {}
            assert_eq!(lexer.error_count, 1);
        }
        assert_eq!(handler_called, 1);
    }

    #[test]
    fn test_simple_positions() {
        // TODO: Remember to keep extending this as we go.
        // This one should be easy to write a generated test for, as that test can parse the
        // generated input by line and use that to keep track of positions.
        let input = r#"pool chairs
pool tables
pool noodles"#;
        let table = &[
            (0, Position::untitled(1, 1)),
            (4, Position::untitled(1, 5)),
            (11, Position::untitled(1, 12)),
            (12, Position::untitled(2, 1)),
            (14, Position::untitled(2, 3)),
            (28, Position::untitled(3, 5)),
            (34, Position::untitled(3, 11)),
            (35, Position::untitled(3, 12)),
        ];

        let mut lexer = Lexer::new(input.as_bytes(), None, None);
        for token in &mut lexer {}
        for (pos, expected) in table {
            assert_eq!(lexer.to_position(Pos(*pos)), *expected);
        }
    }

    #[test]
    fn test_comment() {
        let table: &[(&str, &[&str])] = &[
            ("# to the end", &["# to the end"]),
            (" a # comment", &["# comment"]),
            (
                r#"pool chairs
# a comment
pool useful # another comment
# pool nachos
"#,
                &["# a comment", "# another comment", "# pool nachos"],
            ),
        ];

        for (input, expected_comments) in table {
            let mut expected_iter = expected_comments.iter();
            let res = parse_and_slice(input);
            for token in res {
                match token {
                    Token::Comment(slice) => {
                        let expectation = expected_iter
                            .next()
                            .expect("Got more comments than expected");
                        let actual = std::str::from_utf8(slice).unwrap();
                        assert_eq!(&actual, expectation);
                    }
                    _ => {}
                };
            }
            assert!(
                expected_iter.next().is_none(),
                "Did not get as many comments as expected"
            );
        }
    }

    // TODO: Focus on simple errors and positions next.

    /*
        #[test]
        fn test_basic_rule() {
            let input = r#"
            rule cc
                command = gcc -o $out -c $in
    "#;
            let lexer = Lexer::new(input.as_bytes(), None, None);
            lexer.lex();
        }

            #[test]
            fn test_chars() {
                let input = r#"
                :||=
                "#;
                let lexer = Lexer::new(input);
                assert_eq!(
                    lexer.lex().collect::<Vec<Token>>(),
                    vec![Token::Colon, Token::Pipe2, Token::Equals]
                );
            }
        */
}
