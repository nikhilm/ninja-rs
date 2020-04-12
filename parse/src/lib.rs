#![feature(is_sorted)]

extern crate ninja_desc;

use std::{
    collections::HashMap,
    fmt::{Display, Formatter},
};

use ninja_desc::BuildDescription;

mod desc;
mod lexer;

use desc::DescriptionBuilder;
use lexer::{Lexer, Position, Token};

#[derive(Debug)]
struct Rule<'a> {
    name: &'a [u8],
    command: &'a [u8],
}

// TODO:
// var evaluation

/// Right now a real simple container for rule lookup since we don't have anything except `command`
/// and variable support.
#[derive(Debug)]
struct Env<'a> {
    rules: HashMap<&'a [u8], Rule<'a>>,
}

impl<'a> Env<'a> {
    fn new() -> Env<'a> {
        Env {
            rules: HashMap::new(),
        }
    }

    fn add_rule(&mut self, rule: Rule<'a>) {
        self.rules.insert(rule.name, rule);
    }

    fn lookup_rule<'b, S: Into<&'b [u8]>>(&self, name: S) -> Option<&Rule<'a>> {
        self.rules.get(name.into())
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
    env: Env<'a>,
    builder: DescriptionBuilder,
}

impl<'a, 'b> Parser<'a, 'b> {
    pub fn new(input: &[u8], filename: Option<String>) -> Parser {
        Parser {
            lexer: Lexer::new(input, filename, None),
            env: Env::new(),
            builder: DescriptionBuilder::new(),
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

    fn parse_rule(&mut self) -> Result<(), ParseError> {
        let identifier = self.expect_identifier()?;
        self.discard_newline()?;
        // TODO: Do all the scoping and env stuff.
        self.discard_indent()?;
        let (var, value) = self.read_assignment()?;
        if var != "command".as_bytes() {
            todo!("Don't know how to handle anything except command");
        }
        self.env.add_rule(Rule {
            name: identifier.value(),
            command: value,
        });
        Ok(())
    }

    fn parse_build(&mut self) -> Result<(), ParseError> {
        // TODO: Support all kinds of optional outputs and dependencies.
        #[derive(Debug, PartialEq, Eq)]
        enum State {
            ReadFirstOutput,
            ReadRemainingOutputs,
            ReadRule,
            ReadInputs,
        };

        // I'd have really liked a Vec<&[u8]> here and then converting to an owned vec at the last
        // minute in the edge builder, but haven't figured an ergonomic way for that yet.
        let mut outputs: Vec<Vec<u8>> = Vec::new();
        let mut inputs: Vec<Vec<u8>> = Vec::new();
        let mut rule = None;
        let mut state = State::ReadFirstOutput;
        let mut first_line_pos = None;
        while let Some((token, pos)) = self.lexer.next() {
            if first_line_pos.is_none() {
                first_line_pos = Some(pos);
            }
            match state {
                State::ReadFirstOutput => match token {
                    Token::Path(v) => {
                        eprintln!("Got first output path {}", std::str::from_utf8(v).unwrap());
                        outputs.push(v.into());
                        state = State::ReadRemainingOutputs;
                    }
                    _ => {
                        return Err(ParseError::new(
                            "Expected at least one output for build",
                            pos,
                            &self.lexer,
                        ));
                    }
                },
                State::ReadRemainingOutputs => match token {
                    Token::Path(v) => {
                        eprintln!(
                            "Got another output path {}",
                            std::str::from_utf8(v).unwrap()
                        );
                        outputs.push(v.into());
                        state = State::ReadRemainingOutputs;
                    }
                    Token::Colon => {
                        state = State::ReadRule;
                    }
                    _ => {
                        return Err(ParseError::new(
                            format!("Expected another output or {}, got {}", Token::Colon, token),
                            pos,
                            &self.lexer,
                        ));
                    }
                },
                State::ReadRule => match token {
                    Token::Identifier(v) => {
                        eprintln!("Got rule name {}", std::str::from_utf8(v).unwrap());
                        rule = self.env.lookup_rule(v);
                        if rule.is_none() {
                            return Err(ParseError::new(
                                format!("Unknown rule {}", token),
                                pos,
                                &self.lexer,
                            ));
                        }
                        state = State::ReadInputs;
                    }
                    _ => {
                        return Err(ParseError::new(
                            format!("Expected rule name, got {}", token),
                            pos,
                            &self.lexer,
                        ));
                    }
                },
                State::ReadInputs => match token {
                    Token::Path(v) => {
                        eprintln!("Got input path {}", std::str::from_utf8(v).unwrap());
                        inputs.push(v.into());
                    }
                    Token::Newline => {
                        break;
                    }
                    _ => {
                        return Err(ParseError::new(
                            format!("Expected input or {}, got {}", Token::Newline, token),
                            pos,
                            &self.lexer,
                        ));
                    }
                },
            }
        }

        // TODO: Read remaining lines as bindings as long as indents are encountered.

        // EOF is OK as long as our state machine is done.
        if state == State::ReadInputs {
            let result = self
                .builder
                .new_edge()
                .add_outputs(outputs) // TODO: affected by top level vars
                // probably eval the outputs and inputs at parse time
                .add_inputs(inputs) // TODO: affected by top level vars
                .finish(&self.env, rule.as_ref().unwrap());

            if let Err(conflict) = result {
                Err(ParseError::new(
                    format!(
                        "Same output {} produced by multiple build edges",
                        std::str::from_utf8(&conflict.0).expect("utf8"),
                    ),
                    first_line_pos.expect("a pos"),
                    &self.lexer,
                ))
            } else {
                Ok(())
            }
        } else {
            Err(ParseError::eof(
                "unexpected EOF in the middle of a build edge",
                &self.lexer,
            ))
        }
    }

    pub fn parse(mut self) -> Result<BuildDescription, ParseError> {
        while let Some((token, _pos)) = self.lexer.next() {
            match token {
                Token::Rule => {
                    self.parse_rule()?;
                }
                Token::Build => {
                    self.parse_build()?;
                }
                Token::Newline => {}
                _ => {
                    eprintln!("Unhandled token {:?}", token);
                }
            }
        }
        Ok(self.builder.finish())
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
        let parser = Parser::new(input.as_bytes(), None);
        eprintln!("{:?}", parser.parse().expect("valid parse"));
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
            let _ = parser.parse().expect("valid parse");
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
