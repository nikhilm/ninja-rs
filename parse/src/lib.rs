#![feature(is_sorted)]

use std::fmt::{Display, Formatter};

use thiserror::Error;

pub mod ast;
mod lexer;

use ast::*;
use lexer::{Lexeme, Lexer, LexerError, Position};

#[derive(Debug, Error)]
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
            position,
            line: owned_line,
            message: msg.into(),
        }
    }

    fn eof<S: Into<String>>(msg: S, lexer: &Lexer) -> ParseError {
        let pos = lexer.last_pos();
        ParseError::new(msg, pos, lexer)
    }

    fn from_lexer_error(err: LexerError, lexer: &Lexer) -> ParseError {
        match err {
            LexerError::UnexpectedEof(pos) => ParseError::new("Unexpected EOF", pos, lexer),
            LexerError::IllegalCharacter(pos, ch) => {
                ParseError::new(format!("Illegal character {}", ch), pos, lexer)
            }
            LexerError::NotAnIdentifier(pos, ch) => {
                ParseError::new(format!("Not an identifier {}", ch), pos, lexer)
            }
            LexerError::MissingParen(pos) => {
                ParseError::new("Expected closing parentheses '}}'", pos, lexer)
            }
        }
    }
}

impl Display for ParseError {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        write!(
            f,
            "{filename}:{lineno}:{col}: {msg}\n{line}\n{indent}^ near here",
            filename = self.position.filename.as_deref().unwrap_or(""),
            lineno = self.position.line,
            col = self.position.column,
            msg = self.message,
            line = self.line,
            indent = " ".repeat(self.position.column.saturating_sub(1)),
        )
    }
}

pub struct Parser<'a> {
    lexer: Lexer<'a>,
}

impl<'a> Parser<'a> {
    pub fn new(input: &[u8], filename: Option<String>) -> Parser {
        Parser {
            lexer: Lexer::new(input, filename),
        }
    }

    fn handle_eof(
        &mut self,
        msg_type: &'static str,
    ) -> Result<Result<(Lexeme<'a>, lexer::Pos), LexerError>, ParseError> {
        self.lexer
            .next()
            .ok_or_else(|| ParseError::eof(format!("Expected {}, got EOF", msg_type), &self.lexer))
    }

    fn expect_identifier(&mut self) -> Result<Lexeme<'a>, ParseError> {
        self.handle_eof("identifier").and_then(|res| {
            res.map_err(|lex_err| ParseError::from_lexer_error(lex_err, &self.lexer))
                .and_then(|(token, pos)| match token {
                    Lexeme::Identifier(_) => Ok(token),
                    _ => Err(ParseError::new(
                        format!("Expected identifier, got {}", token),
                        pos,
                        &self.lexer,
                    )),
                })
        })
    }

    fn expect_value(&mut self) -> Result<Lexeme<'a>, ParseError> {
        self.handle_eof("literal").and_then(|res| {
            res.map_err(|lex_err| ParseError::from_lexer_error(lex_err, &self.lexer))
                .and_then(|(token, pos)| match token {
                    Lexeme::Literal(_) => Ok(token),
                    _ => Err(ParseError::new(
                        format!("Expected literal, got {}", token),
                        pos,
                        &self.lexer,
                    )),
                })
        })
    }

    fn discard_indent(&mut self) -> Result<(), ParseError> {
        self.handle_eof("indent").and_then(|res| {
            res.map_err(|lex_err| ParseError::from_lexer_error(lex_err, &self.lexer))
                .and_then(|(token, pos)| match token {
                    Lexeme::Indent => Ok(()),
                    _ => Err(ParseError::new(
                        format!("Expected indent, got {}", token),
                        pos,
                        &self.lexer,
                    )),
                })
        })
    }

    fn discard_newline(&mut self) -> Result<(), ParseError> {
        self.handle_eof("newline").and_then(|res| {
            res.map_err(|lex_err| ParseError::from_lexer_error(lex_err, &self.lexer))
                .and_then(|(token, pos)| match token {
                    Lexeme::Newline => Ok(()),
                    _ => Err(ParseError::new(
                        format!("Expected newline, got {}", token),
                        pos,
                        &self.lexer,
                    )),
                })
        })
    }

    fn discard_assignment(&mut self) -> Result<(), ParseError> {
        self.handle_eof("=").and_then(|res| {
            res.map_err(|lex_err| ParseError::from_lexer_error(lex_err, &self.lexer))
                .and_then(|(token, pos)| match token {
                    Lexeme::Equals => Ok(()),
                    _ => Err(ParseError::new(
                        format!("Expected =, got {}", token),
                        pos,
                        &self.lexer,
                    )),
                })
        })
    }

    fn read_assignment(&mut self) -> Result<(&'a [u8], &'a [u8]), ParseError> {
        let var = self.expect_identifier()?;
        self.discard_assignment()?;
        let value = self.expect_value()?;
        Ok((var.value(), value.value()))
    }

    fn parse_rule(&mut self) -> Result<Rule<'a>, ParseError> {
        let identifier = self.expect_identifier()?;
        self.discard_newline()?;
        self.discard_indent()?;
        let (var, value) = self.read_assignment()?;
        // TODO: Move this to a semantic pass.
        if var != b"command" {
            todo!("Don't know how to handle anything except command");
        }
        Ok(Rule {
            name: identifier.value(),
            command: value,
        })
    }

    fn parse_build(&mut self) -> Result<Build<'a>, ParseError> {
        // TODO: Support all kinds of optional outputs and dependencies.
        #[derive(Debug, PartialEq, Eq)]
        enum Read {
            FirstOutput,
            RemainingOutputs,
            Rule,
            Inputs,
        };

        // I'd have really liked a Vec<&[u8]> here and then converting to an owned vec at the last
        // minute in the edge builder, but haven't figured an ergonomic way for that yet.
        let mut outputs: Vec<&[u8]> = Vec::new();
        let mut inputs: Vec<&[u8]> = Vec::new();
        let mut rule = None;
        let mut state = Read::FirstOutput;
        let mut first_line_pos = None;
        while let Some(result) = self.lexer.next() {
            let (token, pos) =
                result.map_err(|lex_err| ParseError::from_lexer_error(lex_err, &self.lexer))?;
            if first_line_pos.is_none() {
                first_line_pos = Some(pos);
            }
            match state {
                Read::FirstOutput => match token {
                    Lexeme::Literal(v) => {
                        outputs.push(v);
                        state = Read::RemainingOutputs;
                    }
                    _ => {
                        return Err(ParseError::new(
                            "Expected at least one output for build",
                            pos,
                            &self.lexer,
                        ));
                    }
                },
                Read::RemainingOutputs => match token {
                    Lexeme::Literal(v) => {
                        outputs.push(v);
                        state = Read::RemainingOutputs;
                    }
                    Lexeme::Colon => {
                        state = Read::Rule;
                    }
                    _ => {
                        return Err(ParseError::new(
                            format!(
                                "Expected another output or {}, got {}",
                                Lexeme::Colon,
                                token
                            ),
                            pos,
                            &self.lexer,
                        ));
                    }
                },
                Read::Rule => match token {
                    Lexeme::Identifier(v) => {
                        rule = Some(v);
                        state = Read::Inputs;
                    }
                    _ => {
                        return Err(ParseError::new(
                            format!("Expected rule name, got {}", token),
                            pos,
                            &self.lexer,
                        ));
                    }
                },
                Read::Inputs => match token {
                    Lexeme::Literal(v) => {
                        inputs.push(v);
                    }
                    Lexeme::Newline => {
                        break;
                    }
                    _ => {
                        return Err(ParseError::new(
                            format!("Expected input or {}, got {}", Lexeme::Newline, token),
                            pos,
                            &self.lexer,
                        ));
                    }
                },
            }
        }

        // TODO: Read remaining lines as bindings as long as indents are encountered.

        // EOF is OK as long as our state machine is done.
        if state == Read::Inputs {
            Ok(Build {
                rule: rule.take().unwrap(),
                inputs,
                outputs,
            })
        } else {
            Err(ParseError::eof(
                "unexpected EOF in the middle of a build edge",
                &self.lexer,
            ))
        }
    }

    pub fn parse(mut self) -> Result<Description<'a>, ParseError> {
        let mut description = Description {
            rules: Vec::new(),
            builds: Vec::new(),
        };
        while let Some(result) = self.lexer.next() {
            let (token, _pos) =
                result.map_err(|lex_err| ParseError::from_lexer_error(lex_err, &self.lexer))?;
            match token {
                Lexeme::Rule => {
                    description.rules.push(self.parse_rule()?);
                }
                Lexeme::Build => {
                    description.builds.push(self.parse_build()?);
                }
                Lexeme::Newline => {}
                Lexeme::Comment(_) => {}
                _ => {
                    todo!("Unhandled token {:?}", token);
                }
            }
        }
        Ok(description)
    }
}

#[cfg(test)]
mod parser_test {
    use super::Parser;
    use insta::assert_debug_snapshot;

    #[test]
    fn test_simple() {
        let input = r#"
rule cc
    command = gcc -c foo.c

build foo.o: cc foo.c"#;
        // TODO: The parser needs some mechanism to load other "files" when includes or subninjas
        // are encountered.
        let parser = Parser::new(input.as_bytes(), None);
        let ast = parser.parse().expect("valid parse");
        assert_debug_snapshot!(ast);
    }

    #[test]
    fn test_rule_identifier_fail() {
        for (input, expected_col) in &[("rule cc:", 8), ("rule", 5), ("rule\n", 5)] {
            let parser = Parser::new(input.as_bytes(), None);
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
            let parser = Parser::new(input.as_bytes(), None);
            let err = parser.parse().unwrap_err();
            assert_eq!(err.position.line, 2);
            assert_eq!(err.position.column, *expected_col);
            assert!(err.message.contains(expected_token));
        }
    }

    #[test]
    fn test_build_no_bindings() {
        for input in &[
            "build foo.o: touch",
            "build foo.o foo.p: touch",
            "build foo.o foo.p foo.q: touch",
            "build foo.o foo.p: touch inp1 inp2",
            r#"build foo.o foo.p: touch inp1 inp2
build bar.o: touch inp3"#,
            r#"build foo.o foo.p: touch inp1 inp2
rule other
  command = gcc"#,
        ] {
            let with_rule = format!(
                r#"
rule touch
  command = touch
{}"#,
                input
            );
            let parser = Parser::new(with_rule.as_bytes(), None);
            let ast = parser.parse().expect("valid parse");
            assert_debug_snapshot!(ast);
        }
    }

    #[test]
    fn test_build_fail_first_line() {
        for input in &[
            "build", // just bad
            r#"build
"#, // just bad
            "build: touch", // missing output
            "build foo.o touch", // no colon
            "build foo.o: ", // no rule
        ] {
            let parser = Parser::new(input.as_bytes(), None);
            let _ = parser.parse().expect_err("parse should fail");
        }
    }
}
