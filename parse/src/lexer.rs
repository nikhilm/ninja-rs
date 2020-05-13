use std::fmt::{Debug, Display, Formatter};

/// Reflects a position in the stream. This can be translated to a line+column Position using
/// Lexer::to_position.
#[derive(Copy, Clone)]
pub struct Pos(usize); // This way, it is only possible to obtain a Pos from a token/error.

#[derive(Debug, PartialEq, Eq)]
pub struct Position {
    pub filename: Option<String>, // TODO: &str; also, comparing Eq using filenames does not make sense.
    pub line: usize,
    pub column: usize,
}

impl Position {
    fn new(filename: Option<String>, line: usize, column: usize) -> Position {
        Position {
            filename,
            line,
            // Either we are in a state that requires reading arbitrary input, or we are expecting
            // to match the beginning of a declaration/keyword/identifier.
            column,
        }
    }

    #[cfg(test)]
    fn untitled(line: usize, column: usize) -> Position {
        Position {
            filename: None,
            line,
            column,
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
    Literal(&'a [u8]),
    Newline,
    // Yes, parser knowledge leaking here.
    Path(&'a [u8]),
    Pipe,
    Pipe2,
    Pool,
    Rule,
    Subninja,
}

impl<'a> Display for Token<'a> {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match *self {
                Token::Build => "build",
                Token::Colon => ":",
                Token::Default => "default",
                Token::Escape => "$",
                Token::Equals => "=",
                Token::Identifier(_) => "identifier",
                Token::Illegal(_) => "illegal",
                Token::Comment(_) => "comment",
                Token::Include => "include",
                Token::Indent => "indent",
                Token::Literal(_) => "literal",
                Token::Newline => "newline",
                Token::Path(_) => "path",
                Token::Pipe => "|",
                Token::Pipe2 => "||",
                Token::Pool => "pool",
                Token::Rule => "rule",
                Token::Subninja => "subninja",
            }
        )
    }
}

impl<'a> Token<'a> {
    #[cfg(test)]
    pub fn is_identifier(&self) -> bool {
        match *self {
            Token::Identifier(_) => true,
            _ => false,
        }
    }

    #[cfg(test)]
    pub fn is_path(&self) -> bool {
        match *self {
            Token::Path(_) => true,
            _ => false,
        }
    }

    #[cfg(test)]
    pub fn is_literal(&self) -> bool {
        match *self {
            Token::Literal(_) => true,
            _ => false,
        }
    }

    pub fn value(&self) -> &'a [u8] {
        match *self {
            Token::Comment(v) | Token::Identifier(v) | Token::Literal(v) | Token::Path(v) => v,
            _ => panic!("Incorrect token type"),
        }
    }
}

#[derive(Debug, PartialEq, Eq)]
enum LexerMode {
    Default,
    PathMode,
    ValueMode,
    BuildRuleMode,
}

pub struct Lexer<'a> {
    data: &'a [u8],
    filename: Option<String>,
    ch: u8,
    offset: usize,
    next_offset: usize,
    // consider using `smallvec` later.
    line_offsets: Vec<usize>,
    lexer_mode: LexerMode,
    pub error_count: u32,
}

impl<'a> Lexer<'a> {
    pub fn new(data: &'a [u8], filename: Option<String>) -> Lexer<'a> {
        let mut ch = 0;
        if data.len() > 0 {
            ch = data[0];
        }
        Lexer {
            data,
            filename,
            ch,
            offset: 0,
            next_offset: 1,
            line_offsets: vec![0],
            lexer_mode: LexerMode::Default,
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

    fn error(&mut self, _pos: Pos, _reason: &str) {
        self.error_count += 1;
    }

    fn skip_horizontal_whitespace(&mut self) {
        while self.ch == (' ' as u8) || self.ch == ('\t' as u8) {
            self.advance();
        }
    }

    fn is_permitted_identifier_char(ch: u8) -> bool {
        ch.is_ascii_alphanumeric() || ch == '_' as u8
    }

    fn read_identifier(&mut self, pos: usize) -> Token<'a> {
        assert!(pos < self.data.len());
        // The Ninja manual doesn't really define what an identifier is. Since we need to handle
        // paths, we keep going until whitespace.

        let span_start = pos;
        let mut span_end = self.offset; // We've already been advanced and this is exclusive.
        while Lexer::is_permitted_identifier_char(self.ch) {
            span_end += 1;
            self.advance();
        }
        Token::Identifier(&self.data[span_start..span_end])
    }

    fn lookup_keyword(&mut self, ident: Token<'a>) -> Token<'a> {
        match ident {
            Token::Identifier(slice) => match slice {
                // Know a better way than this? as_bytes() is not allowed here.
                [98, 117, 105, 108, 100] => {
                    self.lexer_mode = LexerMode::PathMode;
                    Token::Build
                }
                [100, 101, 102, 97, 117, 108, 116] => {
                    self.lexer_mode = LexerMode::PathMode;
                    Token::Default
                }
                [105, 110, 99, 108, 117, 100, 101] => {
                    self.lexer_mode = LexerMode::PathMode;
                    Token::Include
                }
                [112, 111, 111, 108] => Token::Pool,
                [114, 117, 108, 101] => Token::Rule,
                [115, 117, 98, 110, 105, 110, 106, 97] => {
                    self.lexer_mode = LexerMode::PathMode;
                    Token::Subninja
                }
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

    /// May only be called once the stream is consumed, to ensure we got line numbers right when a
    /// conversion to Position is requested.
    pub fn last_pos(&self) -> Pos {
        assert!(self.done());
        Pos(self.data.len())
    }

    pub fn to_position(&self, pos: Pos) -> Position {
        // maybe a consumed Lexer _should_ return some new object? that has line offsets and error
        // things populated?
        assert!(self.line_offsets.is_sorted());
        if pos.0 > self.data.len() {
            panic!("position {} past end of data {}", pos.0, self.data.len());
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

    /// Panics if position.line is not valid.
    pub fn retrieve_line(&self, position: &Position) -> &'a [u8] {
        assert!(position.line >= 1 && position.line <= self.line_offsets.len());
        let idx = position.line - 1;
        let start = self.line_offsets[idx];
        let end = if idx == self.line_offsets.len() - 1 {
            // Last element.
            // Either we haven't parsed a newline yet, or it is EOF.
            let mut i = start;
            while i < self.data.len() {
                // We could populate line offsets here, but since this is only called on errors, it
                // isn't worth it.
                if self.data[i] == ('\n' as u8) {
                    break;
                }
                i += 1;
            }
            i
        } else {
            // Subtract 1 to exclude the newline itself.
            // We are actually guaranteed that line_offsets[idx+1] is never 0, but lets be safe.
            self.line_offsets[idx + 1].saturating_sub(1)
        };

        &self.data[start..end]
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
     * Ninja lexing is context-sensitive. Sometimes we are reading keywords, sometimes identifiers,
     * sometimes paths and sometimes strings that can have escape sequences and '$'.
     */
    fn read_literal_or_ident(&mut self, pos: usize) -> Option<Token<'a>> {
        assert!(pos < self.data.len());
        let ch = self.data[pos];
        match &self.lexer_mode {
            LexerMode::Default | LexerMode::BuildRuleMode => {
                if Lexer::is_permitted_identifier_char(ch) {
                    let ident = self.read_identifier(pos);
                    if self.lexer_mode == LexerMode::BuildRuleMode {
                        self.lexer_mode = LexerMode::PathMode;
                    }
                    Some(self.lookup_keyword(ident))
                } else {
                    None
                }
            }
            LexerMode::PathMode => {
                // parse the next "space separated" filename, which can include escaped colons.
                // variables are not expanded here.
                Some(self.read_path(pos))
            }
            LexerMode::ValueMode => Some(self.read_literal(pos)),
        }
    }

    fn read_path(&mut self, pos: usize) -> Token<'a> {
        assert!(pos < self.data.len());
        let start = pos;
        let mut end = self.offset;
        loop {
            // This is effectively peeking.
            // If we want to stop processing, at say ':', we will simply bail and the next call to
            // next() will proceed from there.
            match self.ch as char {
                '$' => {
                    todo!("escape sequences are not implemented!");
                }
                ' ' => {
                    // Done with this path.
                    break;
                }
                '|' => {
                    todo!("Implicit outs/deps not supported!");
                }
                '\n' => {
                    // Done with this path. also switch modes.
                    self.lexer_mode = LexerMode::Default;
                    break;
                }
                // Only expect to encounter this in `build` declarations.
                // The parser will take care if that does not happen.
                ':' => {
                    // Separate from default because after reading the rule, we need to go back
                    // to PathMode.
                    self.lexer_mode = LexerMode::BuildRuleMode;
                    break;
                }
                _ => {
                    end += 1;
                    if self.advance().is_none() {
                        break;
                    }
                }
            }
        }
        Token::Path(&self.data[start..end])
    }

    fn read_literal(&mut self, pos: usize) -> Token<'a> {
        assert!(pos < self.data.len());
        let start = pos;
        let mut end = self.offset;
        loop {
            match self.ch as char {
                '$' => {
                    todo!("escape sequences are not implemented!");
                }
                '\n' => {
                    // Done with this literal. also switch modes.
                    self.lexer_mode = LexerMode::Default;
                    break;
                }
                _ => {
                    end += 1;
                    if self.advance().is_none() {
                        break;
                    }
                }
            }
        }
        Token::Literal(&self.data[start..end])
    }
}

impl<'a> Debug for Lexer<'a> {
    fn fmt(&self, fmt: &mut Formatter) -> std::fmt::Result {
        fmt.debug_struct("Lexer")
            .field("filename", &self.filename)
            .field("ch", &self.ch)
            .field("offset", &self.offset)
            .field("next_offset", &self.next_offset)
            .field("lexer_mode", &self.lexer_mode)
            .field("error_count", &self.error_count)
            .finish()
    }
}

type TokenPos<'a> = (Token<'a>, Pos);

impl<'a> Iterator for Lexer<'a> {
    type Item = TokenPos<'a>;
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
        // There is only one reason this loop exists, which is to handle skipping non-indent
        // whitespace. everything else should never come back here.
        loop {
            if self.done() {
                return None;
            }

            let pos = Pos(self.offset);
            let ch = self.ch;

            if ch == ' ' as u8 || ch == '\t' as u8 {
                // If this marks the beginning of the current line, consume all whitespace as an indent,
                // otherwise skip horizontal whitespace.
                let is_indent = self.line_offsets[self.line_offsets.len() - 1] == pos.0;
                self.skip_horizontal_whitespace();
                if is_indent {
                    return Some((Token::Indent, pos));
                } else {
                    continue;
                }
            }

            // Always make progress.
            let next = self.advance();
            return match ch as char {
                // TODO: Windows line ending support.
                // Also not sure if yielding a newline token in the general case really makes
                // sense. Ninja is sensitive about that only in certain cases.
                '\n' => {
                    self.record_line();
                    Some((Token::Newline, pos))
                }
                '=' => {
                    self.lexer_mode = LexerMode::ValueMode;
                    Some((Token::Equals, pos))
                }
                ':' => Some((Token::Colon, pos)),
                '|' => {
                    if let Some(c) = next {
                        if c == ('|' as u8) {
                            self.advance();
                            Some((Token::Pipe2, pos))
                        } else {
                            Some((Token::Pipe, pos))
                        }
                    } else {
                        Some((Token::Pipe, pos))
                    }
                }
                '$' => Some((Token::Escape, pos)),
                // Ninja only allows comments on newlines, so the other modes treat this as a
                // literal. we may want a warning or something.
                '#' => Some((self.read_comment(), pos)),
                _ => self
                    .read_literal_or_ident(pos.0)
                    .map(|x| (x, pos))
                    .or_else(|| {
                        let err = format!("Unexpected character: {}", ch as char);
                        self.error(pos, &err);
                        Some((Token::Illegal(ch), pos))
                    }),
            };
        }
    }
}

#[cfg(test)]
mod test {
    use super::{Lexer, Pos, Position, Token};
    // This may be a good place to use the `insta` crate, but possibly overkill as well.

    fn parse_and_slice(input: &str) -> Vec<Token> {
        let lexer = Lexer::new(input.as_bytes(), None);
        lexer.map(|(token, _pos)| token).collect::<Vec<Token>>()
    }

    fn readable_byte_compare(actual: &[u8], expected: &str) {
        if actual != expected.as_bytes() {
            panic!(
                "Expected: {}, got {}",
                expected,
                std::str::from_utf8(actual).unwrap()
            );
        }
    }

    fn check_identifier(token: &Token, expected: &str) {
        assert!(token.is_identifier());
        readable_byte_compare(token.value(), expected);
    }

    fn check_path(token: &Token, expected: &str) {
        assert!(token.is_path());
        readable_byte_compare(token.value(), expected);
    }

    fn check_literal(token: &Token, expected: &str) {
        assert!(token.is_literal());
        readable_byte_compare(token.value(), expected);
    }

    #[test]
    fn test_simple_colon() {
        assert_eq!(&parse_and_slice(":"), &[Token::Colon]);
    }

    #[test]
    fn test_pool_simple() {
        let stream = parse_and_slice("pool chairs");
        assert_eq!(stream[0], Token::Pool);
        check_identifier(&stream[1], "chairs");
    }

    #[test]
    fn test_error_triggered() {
        // This interface is not very ergonomic...
        let mut lexer = Lexer::new("pool )".as_bytes(), None);
        for _token in &mut lexer {}
        assert_eq!(lexer.error_count, 1);
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

        let mut lexer = Lexer::new(input.as_bytes(), None);
        for _token in &mut lexer {}
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

    #[test]
    fn test_rule_line() {
        let res = parse_and_slice("rule cc");
        assert_eq!(res[0], Token::Rule);
        check_identifier(&res[1], "cc");
    }

    // The non-build kinds.
    #[test]
    fn test_simple_pathmodes() {
        let is_keyword = |k: &Token| match *k {
            Token::Subninja | Token::Include | Token::Default => true,
            _ => false,
        };

        let table = &["subninja apath", "include apath", "default apath"];
        for test in table {
            let res = parse_and_slice(test);
            assert_eq!(res.len(), 2);
            assert!(is_keyword(&res[0]));
            check_path(&res[1], "apath");
        }
    }

    #[test]
    fn test_build_simple() {
        let res = parse_and_slice("build foo.o: cc foo.c");
        assert_eq!(res.len(), 5);
        assert_eq!(res[0], Token::Build);
        check_path(&res[1], "foo.o");
        assert_eq!(res[2], Token::Colon);
        check_identifier(&res[3], "cc");
        check_path(&res[4], "foo.c");
    }

    #[test]
    fn test_simple_rule() {
        let res = parse_and_slice(
            r#"rule cc
    command = gcc"#,
        );
        assert_eq!(res[0], Token::Rule);
        check_identifier(&res[1], "cc");
        assert_eq!(&res[2..4], &[Token::Newline, Token::Indent]);
        check_identifier(&res[4], "command");
        assert_eq!(res[5], Token::Equals);
        check_literal(&res[6], "gcc");
    }

    #[test]
    fn test_chars() {
        let res = parse_and_slice(
            r#"
                :||=
                "#,
        );
        assert_eq!(
            res,
            vec![
                Token::Newline,
                Token::Indent,
                Token::Colon,
                Token::Pipe2,
                Token::Equals,
                Token::Newline,
                Token::Indent
            ]
        );
    }
}
