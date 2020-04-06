#![feature(is_sorted)]
#![feature(todo_macro)]

use std::fmt::{Display, Formatter};

mod lexer;

use lexer::{Lexer, Position, Token};

#[derive(Debug)]
struct Rule {
    name: String,
    command: String,
}

// TODO: Canonicalization pass
// var evaluation
// lifetimes and graphs in Rust

#[derive(Debug)]
struct BuildEdge {
    // outputs: Vec<
// inputs:
// rule: ownership story
}

#[derive(Debug)]
pub struct BuildDescription {
    // environment: Env, // TODO
    rules: Vec<Rule>, // hashtable?
    build_edges: Vec<BuildEdge>,
    // defaults: Vec<...>, // TODO
}

impl BuildDescription {
    fn new() -> BuildDescription {
        BuildDescription {
            rules: Vec::new(),
            build_edges: Vec::new(),
        }
    }

    fn add_rule(&mut self, rule: Rule) {
        self.rules.push(rule);
    }
}

#[derive(Debug)]
pub struct ParseError {
    position: Position,
    line: String,
    message: String,
}

impl ParseError {
    fn new<S: Into<String>>(msg: S, pos: lexer::Pos, lexer: &Lexer) -> ParseError {
        let position = lexer.to_position(pos);
        let line = lexer.retrieve_line(&position);
        // TODO: Invalid utf8 should trigger nice error.
        let owned_line = std::str::from_utf8(line).expect("utf8").to_owned();
        ParseError {
            position: position,
            line: owned_line,
            message: msg.into(),
        }
    }

    fn eof(msg: &'static str, lexer: &Lexer) -> ParseError {
        let pos = lexer.last_pos();
        ParseError::new(msg, pos, lexer)
    }
}

impl Display for ParseError {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        write!(
            f,
            "{filename}:{lineno}:{col}: {msg}\n{line}\n{indent}^ near here",
            filename = self
                .position
                .filename
                .as_ref()
                .map(|x| x.as_str())
                .unwrap_or(""),
            lineno = self.position.line,
            col = self.position.column,
            msg = self.message,
            line = self.line,
            indent = " ".repeat(self.position.column.saturating_sub(1)),
        )
    }
}

pub struct Parser<'a, 'b> {
    lexer: Lexer<'a, 'b>,
    build_description: BuildDescription,
}

impl<'a, 'b> Parser<'a, 'b> {
    pub fn new(input: &[u8], filename: Option<String>) -> Parser {
        Parser {
            lexer: Lexer::new(input, filename, None),
            build_description: BuildDescription::new(),
        }
    }

    fn expect_identifier(&mut self) -> Result<Token<'a>, ParseError> {
        self.lexer
            .next()
            .ok_or_else(|| ParseError::eof("Expected identifier, got EOF", &self.lexer))
            .and_then(|(token, pos)| match token {
                Token::Identifier(_) => Ok(token),
                _ => Err(ParseError::new(
                    format!("Expected identifier, got {}", token),
                    pos,
                    &self.lexer,
                )),
            })
    }

    fn expect_value(&mut self) -> Result<Token<'a>, ParseError> {
        self.lexer
            .next()
            .ok_or_else(|| ParseError::eof("Expected literal, got EOF", &self.lexer))
            .and_then(|(token, pos)| match token {
                Token::Literal(_) => Ok(token),
                _ => Err(ParseError::new(
                    format!("Expected literal, got {}", token),
                    pos,
                    &self.lexer,
                )),
            })
    }

    fn discard_indent(&mut self) -> Result<(), ParseError> {
        self.lexer
            .next()
            .ok_or_else(|| ParseError::eof("Expected indent, got EOF", &self.lexer))
            .and_then(|(token, pos)| match token {
                Token::Indent => Ok(()),
                _ => Err(ParseError::new(
                    format!("Expected indent, got {}", token),
                    pos,
                    &self.lexer,
                )),
            })
    }

    fn discard_newline(&mut self) -> Result<(), ParseError> {
        self.lexer
            .next()
            .ok_or_else(|| ParseError::eof("Expected newline, got EOF", &self.lexer))
            .and_then(|(token, pos)| match token {
                Token::Newline => Ok(()),
                _ => Err(ParseError::new(
                    format!("Expected newline, got {}", token),
                    pos,
                    &self.lexer,
                )),
            })
    }

    fn discard_assignment(&mut self) -> Result<(), ParseError> {
        self.lexer
            .next()
            .ok_or_else(|| ParseError::eof("Expected =, got EOF", &self.lexer))
            .and_then(|(token, pos)| match token {
                Token::Equals => Ok(()),
                _ => Err(ParseError::new(
                    format!("Expected =, got {}", token),
                    pos,
                    &self.lexer,
                )),
            })
    }

    fn read_assignment(&mut self) -> Result<(&'a [u8], &'a [u8]), ParseError> {
        let var = self.expect_identifier()?;
        self.discard_assignment()?;
        let value = self.expect_value()?;
        Ok((var.value(), value.value()))
    }

    fn token_to_string(token: Token) -> Result<String, ParseError> {
        // TODO: What we would really like is to convert utf8 errors into a position in the token
        // stream and generate a nice error.
        Ok(std::str::from_utf8(token.value()).expect("utf8").to_owned())
    }

    fn parse_rule(&mut self) -> Result<(), ParseError> {
        let identifier = self.expect_identifier()?;
        self.discard_newline()?;
        // TODO: Do all the scoping and env stuff.
        self.discard_indent()?;
        let (var, value) = self.read_assignment()?;
        if var != "command".as_bytes() {
            todo!("Don't know how to handle anything except command");
        }
        self.build_description.add_rule(Rule {
            name: Parser::token_to_string(identifier)?,
            command: std::str::from_utf8(value).expect("utf8").to_owned(),
        });
        Ok(())
    }

    pub fn parse(mut self) -> Result<BuildDescription, ParseError> {
        while let Some((token, pos)) = self.lexer.next() {
            match token {
                Token::Rule => {
                    self.parse_rule()?;
                }
                Token::Newline => {}
                _ => {
                    eprintln!("Unhandled token {:?}", token);
                }
            }
        }
        Ok(self.build_description)
    }
}

#[cfg(test)]
mod parser_test {
    use super::Parser;

    #[test]
    fn test_simple() {
        let input = r#"
rule cc
    command = gcc -c foo.c

build foo.o: cc foo.c"#;
        // TODO: The parser needs some mechanism to load other "files" when includes or subninjas
        // are encountered.
        let mut parser = Parser::new(input.as_bytes(), None);
        parser.parse().expect("valid parse");
    }

    #[test]
    fn test_rule_identifier_fail() {
        for (input, expected_col) in &[("rule cc:", 8), ("rule", 5), ("rule\n", 5)] {
            let mut parser = Parser::new(input.as_bytes(), None);
            let err = parser.parse().unwrap_err();
            assert_eq!(err.position.line, 1);
            assert_eq!(err.position.column, *expected_col);
        }
    }

    #[test]
    fn test_rule_missing_command() {
        for (input, expected_col, expected_token) in &[
            (
                // Expect indent
                r#"rule cc
command"#,
                1,
                "indent",
            ),
            (
                r#"rule cc
  command"#,
                10,
                "=",
            ),
            (
                r#"rule cc
  command ="#,
                12,
                "literal",
            ),
            (
                r#"rule cc
  command="#,
                11,
                "literal",
            ),
        ] {
            let mut parser = Parser::new(input.as_bytes(), None);
            let err = parser.parse().unwrap_err();
            assert_eq!(err.position.line, 2);
            assert_eq!(err.position.column, *expected_col);
            assert!(err.message.contains(expected_token));
        }
    }
}
