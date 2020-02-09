use std::iter::Peekable;
use std::str::Chars;

pub struct Lexer<'a> {
    input: &'a str,
}

#[derive(Debug, PartialEq, Eq)]
pub enum Token {
    Build,
    Colon,
    Default,
    Equals,
    Identifier(String),
    Illegal(char),
    Include,
    Indent,
    Newline,
    Pipe,
    Pipe2,
    Pool,
    Rule,
    Subninja,
}

pub struct LexIterator<'a> {
    input: Peekable<Chars<'a>>,
}

impl<'a> Lexer<'a> {
    pub fn new(data: &str) -> Lexer {
        Lexer { input: data }
    }

    pub fn lex(self) -> LexIterator<'a> {
        LexIterator::new(self)
    }
}

impl<'a> LexIterator<'a> {
    pub fn new(lexer: Lexer) -> LexIterator {
        let mut it = LexIterator {
            input: lexer.input.chars().peekable(),
        };
        it.skip_whitespace();
        it
    }
    fn skip_whitespace(&mut self) {
        while let Some(c) = self.peek_char() {
            if c.is_whitespace() {
                eprintln!("Got ws");
                self.read_char();
            } else {
                break;
            }
        }
    }

    fn read_char(&mut self) -> Option<char> {
        self.input.next()
    }

    fn peek_char(&mut self) -> Option<&char> {
        self.input.peek()
    }

    fn read_identifier(&mut self, first: char) -> Token {
        // wish there was a way to tag as pos.
        // TODO: Make this zero-copy
        let mut identifier = String::new();
        identifier.push(first);
        while let Some(&c) = self.peek_char() {
            if c.is_alphanumeric() {
                identifier.push(c);
                self.read_char();
            } else {
                break;
            }
        }
        Token::Identifier(identifier)
    }

    fn lookup_keyword(ident: Token) -> Token {
        match ident {
            Token::Identifier(ref s) => match s.as_str() {
                "build" => Token::Build,
                "default" => Token::Default,
                "include" => Token::Include,
                "pool" => Token::Pool,
                "rule" => Token::Rule,
                "subninja" => Token::Subninja,
                _ => ident,
            },
            _ => {
                panic!("Expected identifier");
            }
        }
    }
}

impl<'a> Iterator for LexIterator<'a> {
    type Item = Token;
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
    fn next(&mut self) -> Option<Self::Item> {
        if let Some(c) = self.read_char() {
            match c {
                // TODO: Windows line ending support.
                '\n' => {
                    eprintln!("Got newline");
                    Some(Token::Newline)
                }
                '\t' => Some(Token::Illegal('\t')),
                // TODO: Handle indentation.
                '=' => Some(Token::Equals),
                ':' => Some(Token::Colon),
                '|' => {
                    if let Some(c) = self.peek_char() {
                        if *c == '|' {
                            self.read_char();
                            Some(Token::Pipe2)
                        } else {
                            Some(Token::Pipe)
                        }
                    } else {
                        Some(Token::Pipe)
                    }
                }
                '$' => {
                    // TODO: Process escapes.
                    Some(Token::Illegal('$'))
                }
                _ => {
                    if c.is_ascii_alphabetic() {
                        Some(LexIterator::lookup_keyword(self.read_identifier(c)))
                    } else {
                        Some(Token::Illegal(c))
                    }
                }
            }
        } else {
            None
        }
    }
}

#[cfg(test)]
mod test {
    use super::Lexer;
    use super::Token;
    // This may be a good place to use the `insta` crate, but possibly overkill as well.
    #[test]
    fn test_basic_rule() {
        let input = r#"
        rule cc
            command = gcc -o $out -c $in
"#;
        let lexer = Lexer::new(input);
        assert_eq!(
            lexer.lex().collect::<Vec<Token>>(),
            vec![Token::Rule, Token::Identifier("cc".to_owned())]
        );
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
}
