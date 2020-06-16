use std::fmt::{Debug, Display, Formatter};
use thiserror::Error;

/// Reflects a position in the stream. This can be translated to a line+column Position using
/// Lexer::to_position.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
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
pub enum VarRefType {
    WithoutParens,
    WithParens,
}

#[derive(Debug, PartialEq, Eq)]
pub enum Lexeme<'a> {
    Build,
    Colon,
    Default,
    Equals,
    // Keep as a separate token type for now, since we may need it when pretty-printing a
    // description.
    Escape(&'a [u8]),
    Identifier(&'a [u8]),
    Comment(&'a [u8]),
    Include,
    Indent,
    Literal(&'a [u8]),
    Newline,
    Pipe,
    Pipe2,
    Pool,
    Rule,
    Subninja,
    VarRef(VarRefType, &'a [u8]),
}

impl<'a> Display for Lexeme<'a> {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match *self {
                Lexeme::Build => "build",
                Lexeme::Colon => ":",
                Lexeme::Default => "default",
                Lexeme::Escape(_) => "escape",
                Lexeme::Equals => "=",
                Lexeme::Identifier(_) => "identifier",
                Lexeme::Comment(_) => "comment",
                Lexeme::Include => "include",
                Lexeme::Indent => "indent",
                Lexeme::Literal(_) => "literal",
                Lexeme::Newline => "newline",
                Lexeme::Pipe => "|",
                Lexeme::Pipe2 => "||",
                Lexeme::Pool => "pool",
                Lexeme::Rule => "rule",
                Lexeme::Subninja => "subninja",
                Lexeme::VarRef(_, _) => "varref",
            }
        )
    }
}

impl<'a> Lexeme<'a> {
    pub fn value(&self) -> &'a [u8] {
        match *self {
            Lexeme::Comment(v) | Lexeme::Identifier(v) | Lexeme::Literal(v) => v,
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

#[derive(Error, Debug, PartialEq, Eq)]
pub enum LexerError {
    /// Different from the iterator returning None. This means an EOF was encountered while looking
    /// for a valid lexeme. The iterator returns None when a valid lexeme was found and then an EOF
    /// was encountered.
    #[error("Unexpected EOF")]
    UnexpectedEof(Pos),
    #[error("Illegal character")]
    IllegalCharacter(Pos, u8),
    #[error("Expected identifier ([a-zA-Z_-])")]
    NotAnIdentifier(Pos, u8),
    #[error("Missing closing paren '}}'")]
    MissingParen(Pos),
}

type LexerResult<'a> = Result<Lexeme<'a>, LexerError>;

pub struct Lexer<'a> {
    data: &'a [u8],
    filename: Option<String>,
    ch: u8,
    offset: usize,
    next_offset: usize,
    // consider using `smallvec` later.
    line_offsets: Vec<usize>,
    lexer_mode: LexerMode,
}

impl<'a> Lexer<'a> {
    pub fn new(data: &'a [u8], filename: Option<String>) -> Lexer<'a> {
        let ch = if !data.is_empty() { data[0] } else { 0 };
        Lexer {
            data,
            filename,
            ch,
            offset: 0,
            next_offset: 1,
            line_offsets: vec![0],
            lexer_mode: LexerMode::Default,
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

    fn skip_horizontal_whitespace(&mut self) {
        while self.ch == b' ' || self.ch == b'\t' {
            self.advance();
        }
    }

    fn is_permitted_identifier_char(ch: u8) -> bool {
        ch.is_ascii_alphanumeric() || ch == b'_' || ch == b'-'
    }

    fn read_identifier(&mut self, pos: usize) -> Lexeme<'a> {
        assert!(pos < self.data.len());
        let span_start = pos;
        let mut span_end = self.offset; // We've already been advanced and this is exclusive.
        while Lexer::is_permitted_identifier_char(self.ch) {
            span_end += 1;
            self.advance();
        }
        Lexeme::Identifier(&self.data[span_start..span_end])
    }

    fn lookup_keyword(&mut self, ident: Lexeme<'a>) -> Lexeme<'a> {
        match ident {
            Lexeme::Identifier(slice) => match slice {
                // Know a better way than this? as_bytes() is not allowed here.
                [98, 117, 105, 108, 100] => {
                    self.lexer_mode = LexerMode::PathMode;
                    Lexeme::Build
                }
                [100, 101, 102, 97, 117, 108, 116] => {
                    self.lexer_mode = LexerMode::PathMode;
                    Lexeme::Default
                }
                [105, 110, 99, 108, 117, 100, 101] => {
                    self.lexer_mode = LexerMode::PathMode;
                    Lexeme::Include
                }
                [112, 111, 111, 108] => Lexeme::Pool,
                [114, 117, 108, 101] => Lexeme::Rule,
                [115, 117, 98, 110, 105, 110, 106, 97] => {
                    self.lexer_mode = LexerMode::PathMode;
                    Lexeme::Subninja
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
                if self.data[i] == b'\n' {
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

    fn read_comment(&mut self) -> Lexeme<'a> {
        // TODO: Handle \r\n
        let start = self.offset - 1; // Includes the '#' in the comment.
        let mut end = self.offset;
        while !self.done() && self.ch != b'\n' {
            end += 1;
            self.advance();
        }
        Lexeme::Comment(&self.data[start..end])
    }

    /*
     * Ninja lexing is context-sensitive. Sometimes we are reading keywords, sometimes identifiers,
     * sometimes paths and sometimes strings that can have escape sequences and '$'.
     */
    fn read_literal_or_ident(&mut self, pos: usize) -> LexerResult<'a> {
        assert!(pos < self.data.len());
        let ch = self.data[pos];
        match &self.lexer_mode {
            LexerMode::Default | LexerMode::BuildRuleMode => {
                if Lexer::is_permitted_identifier_char(ch) {
                    let ident = self.read_identifier(pos);
                    if self.lexer_mode == LexerMode::BuildRuleMode {
                        self.lexer_mode = LexerMode::PathMode;
                    }
                    Ok(self.lookup_keyword(ident))
                } else {
                    Err(LexerError::NotAnIdentifier(Pos(pos), ch))
                }
            }
            LexerMode::PathMode => {
                // parse the next "space separated" filename, which can include escaped colons.
                self.read_path(pos)
            }
            LexerMode::ValueMode => self.read_literal(pos),
        }
    }

    fn read_path(&mut self, pos: usize) -> LexerResult<'a> {
        assert!(pos < self.data.len());
        let start = pos;
        let mut end = self.offset;
        loop {
            // This is effectively peeking.
            // If we want to stop processing, at say ':', we will simply bail and the next call to
            // next() will proceed from there.
            match self.ch {
                b'$' => {
                    todo!("escape sequences are not implemented!");
                }
                b'\n' => {
                    // Done with this path. also switch modes.
                    self.lexer_mode = LexerMode::Default;
                    break;
                }
                b' ' => {
                    // Done with this path.
                    break;
                }
                b'|' => {
                    todo!("Implicit outs/deps not supported!");
                }
                // Only expect to encounter this in `build` declarations.
                // The parser will take care if that does not happen.
                b':' => {
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
        Ok(Lexeme::Literal(&self.data[start..end]))
    }

    fn read_literal(&mut self, pos: usize) -> LexerResult<'a> {
        assert!(pos < self.data.len());
        let start = pos;
        let mut end = self.offset;
        loop {
            match self.ch {
                b'$' => {
                    // Don't switch modes, since we don't know how to interpret this yet.
                    break;
                }
                b'\n' => {
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
        Ok(Lexeme::Literal(&self.data[start..end]))
    }

    fn read_escape(&mut self, pos: usize) -> Result<Lexeme<'a>, LexerError> {
        assert!(pos < self.data.len());
        assert_eq!(self.data[pos], b'$');
        assert_eq!(pos + 1, self.offset);

        let result = match self.ch {
            b' ' | b'\n' | b'\r' | b'$' | b':' => {
                Ok(Lexeme::Escape(&self.data[self.offset..self.offset + 1]))
            }
            b'{' => {
                let pos = self.offset;
                if self.advance().is_none() {
                    return Err(LexerError::UnexpectedEof(Pos(pos)));
                }
                // This and the next if is kinda ugly.
                if !Lexer::is_permitted_identifier_char(self.ch) {
                    return Err(LexerError::NotAnIdentifier(Pos(self.offset), self.ch));
                }

                let pos = self.offset;
                self.advance();
                let ident = self.read_identifier(pos);

                if self.done() {
                    return Err(LexerError::UnexpectedEof(Pos(self.offset - 1)));
                } else if self.ch != b'}' {
                    return Err(LexerError::MissingParen(Pos(self.offset)));
                }

                Ok(Lexeme::VarRef(VarRefType::WithParens, ident.value()))
            }
            _ if Lexer::is_permitted_identifier_char(self.ch) => {
                let pos = self.offset;
                self.advance();
                let ident = self.read_identifier(pos);
                Ok(Lexeme::VarRef(VarRefType::WithoutParens, ident.value()))
            }
            0 => Err(LexerError::UnexpectedEof(Pos(pos))),
            _ => Err(LexerError::IllegalCharacter(Pos(self.offset), self.ch)),
        };
        // Advance either way so the rest of the lexer can continue;
        self.advance();
        result
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
            .finish()
    }
}

type TokenPos<'a> = (Lexeme<'a>, Pos);

impl<'a> Iterator for Lexer<'a> {
    type Item = Result<TokenPos<'a>, LexerError>;

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

            if ch == b' ' || ch == b'\t' {
                // If this marks the beginning of the current line, consume all whitespace as an indent,
                // otherwise skip horizontal whitespace.
                let is_indent = self.line_offsets[self.line_offsets.len() - 1] == pos.0;
                self.skip_horizontal_whitespace();
                if is_indent {
                    return Some(Ok((Lexeme::Indent, pos)));
                } else {
                    continue;
                }
            }

            // Always make progress.
            let next = self.advance();
            return match ch {
                // TODO: Windows line ending support.
                // Also not sure if yielding a newline token in the general case really makes
                // sense. Ninja is sensitive about that only in certain cases.
                b'\n' => {
                    self.record_line();
                    self.lexer_mode = LexerMode::Default;
                    Some(Ok((Lexeme::Newline, pos)))
                }
                b'=' => {
                    self.lexer_mode = LexerMode::ValueMode;
                    Some(Ok((Lexeme::Equals, pos)))
                }
                b':' => Some(Ok((Lexeme::Colon, pos))),
                b'|' => {
                    if let Some(c) = next {
                        if c == b'|' {
                            self.advance();
                            Some(Ok((Lexeme::Pipe2, pos)))
                        } else {
                            Some(Ok((Lexeme::Pipe, pos)))
                        }
                    } else {
                        Some(Ok((Lexeme::Pipe, pos)))
                    }
                }
                b'$' => Some(self.read_escape(pos.0).map(|x| (x, pos))),
                // Ninja only allows comments on newlines, so the other modes treat this as a
                // literal. we may want a warning or something.
                b'#' => Some(Ok((self.read_comment(), pos))),
                _ => Some(self.read_literal_or_ident(pos.0).map(|x| (x, pos))),
            };
        }
    }
}

#[cfg(test)]
mod test {
    use super::{Lexeme, Lexer, LexerError, Pos, Position};
    // This may be a good place to use the `insta` crate, but possibly overkill as well.

    fn parse_and_slice(input: &str) -> Vec<Result<Lexeme, LexerError>> {
        let lexer = Lexer::new(input.as_bytes(), None);
        lexer.map(|v| v.map(|(token, _pos)| token)).collect()
    }

    fn parse_and_slice_no_error(input: &str) -> Vec<Lexeme> {
        parse_and_slice(input)
            .into_iter()
            .map(|v| v.expect("valid lexeme"))
            .collect()
    }

    #[test]
    fn test_simple_colon() {
        assert_eq!(&parse_and_slice_no_error(":"), &[Lexeme::Colon]);
    }

    #[test]
    fn test_pool_simple() {
        let stream = parse_and_slice_no_error("pool chairs");
        assert_eq!(stream, &[Lexeme::Pool, Lexeme::Identifier(b"chairs")]);
    }

    #[test]
    fn test_error_triggered() {
        // This interface is not very ergonomic...
        let lexemes = parse_and_slice("pool )");
        assert_eq!(
            lexemes,
            &[
                Ok(Lexeme::Pool),
                Err(LexerError::NotAnIdentifier(Pos(5), 41))
            ]
        );
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
            let res = parse_and_slice_no_error(input);
            for token in res {
                match token {
                    Lexeme::Comment(slice) => {
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
        let res = parse_and_slice_no_error("rule cc");
        assert_eq!(res, &[Lexeme::Rule, Lexeme::Identifier(b"cc")]);
    }

    // The non-build kinds.
    #[test]
    fn test_simple_pathmodes() {
        let is_keyword = |k: &Lexeme| match *k {
            Lexeme::Subninja | Lexeme::Include | Lexeme::Default => true,
            _ => false,
        };

        let table = &["subninja apath", "include apath", "default apath"];
        for test in table {
            let res = parse_and_slice_no_error(test);
            assert_eq!(res.len(), 2);
            assert!(is_keyword(&res[0]));
            assert_eq!(res[1], Lexeme::Literal(b"apath"));
        }
    }

    #[test]
    fn test_build_simple() {
        let res = parse_and_slice_no_error("build foo.o: cc foo.c");
        assert_eq!(
            res,
            &[
                Lexeme::Build,
                Lexeme::Literal(b"foo.o"),
                Lexeme::Colon,
                Lexeme::Identifier(b"cc"),
                Lexeme::Literal(b"foo.c")
            ]
        );
    }

    #[test]
    fn test_simple_rule() {
        let res = parse_and_slice_no_error(
            r#"rule cc
    command = gcc"#,
        );
        assert_eq!(
            res,
            &[
                Lexeme::Rule,
                Lexeme::Identifier(b"cc"),
                Lexeme::Newline,
                Lexeme::Indent,
                Lexeme::Identifier(b"command"),
                Lexeme::Equals,
                Lexeme::Literal(b"gcc"),
            ]
        );
    }

    #[test]
    fn test_chars() {
        let res = parse_and_slice_no_error(
            r#"
                :||=
                "#,
        );
        assert_eq!(
            res,
            &[
                Lexeme::Newline,
                Lexeme::Indent,
                Lexeme::Colon,
                Lexeme::Pipe2,
                Lexeme::Equals,
                Lexeme::Newline,
                Lexeme::Indent
            ]
        );
    }

    #[test]
    fn test_newline_when_path_expected() {
        let res = parse_and_slice_no_error(
            r#"rule touch
    command = touch no_inputs.txt

build no_inputs.txt: touch
build next: touch"#,
        );
        assert_eq!(
            res,
            &[
                Lexeme::Rule,
                Lexeme::Identifier(b"touch"),
                Lexeme::Newline,
                Lexeme::Indent,
                Lexeme::Identifier(b"command"),
                Lexeme::Equals,
                Lexeme::Literal(b"touch no_inputs.txt"),
                Lexeme::Newline,
                Lexeme::Newline,
                Lexeme::Build,
                Lexeme::Literal(b"no_inputs.txt"),
                Lexeme::Colon,
                Lexeme::Identifier(b"touch"),
                Lexeme::Newline,
                Lexeme::Build,
                Lexeme::Literal(b"next"),
                Lexeme::Colon,
                Lexeme::Identifier(b"touch"),
            ]
        );
    }

    #[test]
    fn test_escape_in_illegal_pos() {
        let res = parse_and_slice_no_error(
            r#"rule c$ c
            command = touch"#,
        );
        // Totally allowed in the lexer. It is the parser that should complain.
        assert_eq!(
            res,
            &[
                Lexeme::Rule,
                Lexeme::Identifier(b"c"),
                Lexeme::Escape(b" "),
                Lexeme::Identifier(b"c"),
                Lexeme::Newline,
                Lexeme::Indent,
                Lexeme::Identifier(b"command"),
                Lexeme::Equals,
                Lexeme::Literal(b"touch")
            ]
        );
    }

    #[test]
    fn test_escape_literal() {
        let res = parse_and_slice_no_error(
            r#"rule cc
            command = abcd$
ef"#,
        );
        assert_eq!(
            res,
            &[
                Lexeme::Rule,
                Lexeme::Identifier(b"cc"),
                Lexeme::Newline,
                Lexeme::Indent,
                Lexeme::Identifier(b"command"),
                Lexeme::Equals,
                Lexeme::Literal(b"abcd"),
                Lexeme::Escape(b"\n"),
                Lexeme::Literal(b"ef"),
            ]
        );

        let res = parse_and_slice_no_error(
            r#"rule cc
            command = abcd$

rule"#,
        );
        assert_eq!(
            res,
            &[
                Lexeme::Rule,
                Lexeme::Identifier(b"cc"),
                Lexeme::Newline,
                Lexeme::Indent,
                Lexeme::Identifier(b"command"),
                Lexeme::Equals,
                Lexeme::Literal(b"abcd"),
                Lexeme::Escape(b"\n"),
                Lexeme::Newline,
                Lexeme::Rule,
            ]
        );
    }

    #[test]
    fn test_escape_eof() {
        let input = r#"rule cc
            command = abcd$"#;
        let res = parse_and_slice(input);
        assert_eq!(
            res,
            &[
                Ok(Lexeme::Rule),
                Ok(Lexeme::Identifier(b"cc")),
                Ok(Lexeme::Newline),
                Ok(Lexeme::Indent),
                Ok(Lexeme::Identifier(b"command")),
                Ok(Lexeme::Equals),
                Ok(Lexeme::Literal(b"abcd")),
                Err(LexerError::UnexpectedEof(Pos(input.len() - 1))),
            ]
        );

        let input = r#"rule cc
            command = abcd${abcd"#;
        let res = parse_and_slice(input);
        assert_eq!(
            res,
            &[
                Ok(Lexeme::Rule),
                Ok(Lexeme::Identifier(b"cc")),
                Ok(Lexeme::Newline),
                Ok(Lexeme::Indent),
                Ok(Lexeme::Identifier(b"command")),
                Ok(Lexeme::Equals),
                Ok(Lexeme::Literal(b"abcd")),
                Err(LexerError::UnexpectedEof(Pos(input.len() - 1))),
            ]
        );
    }

    #[test]
    #[should_panic]
    fn test_escape_varrefs() {
        todo!();
    }

    #[test]
    #[should_panic]
    fn test_escape_and_lex_modes() {
        // TODO: Make sure path mode is continued/reset based on newlines/colon.
        todo!();
    }
}
