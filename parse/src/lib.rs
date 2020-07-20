#![feature(is_sorted)]

use std::{
    cell::RefCell,
    collections::HashMap,
    fmt::{Display, Formatter},
    rc::Rc,
};

use thiserror::Error;

pub mod ast;
pub mod env;
mod lexer;

use ast::*;
pub use env::Env;
use lexer::{Lexeme, Lexer, LexerError, LexerItem, Position};

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
            LexerError::IllegalCharacter(pos, _ch) => {
                ParseError::new("Illegal character", pos, lexer)
            }
            LexerError::NotAnIdentifier(pos, _ch) => {
                ParseError::new("Expected identifier", pos, lexer)
            }
            LexerError::MissingBrace(pos) => {
                ParseError::new("Expected closing parentheses '}'", pos, lexer)
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

#[derive(Default)]
struct Peeker<'a> {
    peeked: Option<LexerItem<'a>>,
}

impl<'a> Peeker<'a> {
    fn next(&mut self, lexer: &mut Lexer<'a>) -> Option<LexerItem<'a>> {
        if self.peeked.is_some() {
            self.peeked.take()
        } else {
            lexer.next()
        }
    }

    fn peek(&mut self, lexer: &mut Lexer<'a>) -> Option<&LexerItem<'a>> {
        if self.peeked.is_none() {
            self.peeked = self.next(lexer);
        }
        self.peeked.as_ref()
    }
}

pub struct Parser<'a> {
    lexer: Lexer<'a>,
    peeker: Peeker<'a>,
}

impl<'a> Parser<'a> {
    pub fn new(input: &[u8], filename: Option<String>) -> Parser {
        Parser {
            lexer: Lexer::new(input, filename),
            peeker: Default::default(),
        }
    }

    fn handle_eof_and_comments(
        &mut self,
        msg_type: &'static str,
    ) -> Result<Result<(Lexeme<'a>, lexer::Pos), LexerError>, ParseError> {
        loop {
            let item = self.peeker.next(&mut self.lexer);
            if item.is_none() {
                return Err(ParseError::eof(
                    format!("Expected {}, got EOF", msg_type),
                    &self.lexer,
                ));
            } else {
                let item = item.unwrap();
                if let Ok((lexeme, _)) = &item {
                    match lexeme {
                        Lexeme::Comment(_) => continue,
                        _ => return Ok(item),
                    }
                } else {
                    return Ok(item);
                }
            }
        }
    }

    fn expr_to_expr(lexeme: Lexeme<'a>) -> Expr<'a> {
        lexeme.check();
        if let Lexeme::Expr(items) = lexeme {
            Expr(
                items
                    .iter()
                    .map(|item| match item {
                        Lexeme::Literal(v) | Lexeme::Escape(v) => Term::Literal(v),
                        Lexeme::VarRef(_, v) => Term::Reference(v),
                        _ => unreachable!(),
                    })
                    .collect(),
            )
        } else {
            panic!("Unexpected lexeme {}", lexeme);
        }
    }

    fn expect_identifier(&mut self) -> Result<Lexeme<'a>, ParseError> {
        self.handle_eof_and_comments("identifier").and_then(|res| {
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

    fn expect_identifier_eating_indent(&mut self) -> Result<Lexeme<'a>, ParseError> {
        let mut stop = true;
        loop {
            let result = self.handle_eof_and_comments("identifier").and_then(|res| {
                res.map_err(|lex_err| ParseError::from_lexer_error(lex_err, &self.lexer))
                    .and_then(|(token, pos)| match token {
                        Lexeme::Indent => {
                            stop = false;
                            Ok(token)
                        }
                        Lexeme::Identifier(_) => Ok(token),
                        _ => Err(ParseError::new(
                            format!("Expected identifier, got {}", token),
                            pos,
                            &self.lexer,
                        )),
                    })
            });
            if stop {
                return result;
            }
            stop = true;
        }
    }

    fn expect_value(&mut self) -> Result<Expr<'a>, ParseError> {
        self.handle_eof_and_comments("value").and_then(|res| {
            res.map_err(|lex_err| ParseError::from_lexer_error(lex_err, &self.lexer))
                .and_then(|(token, pos)| match token {
                    Lexeme::Expr(_) => Ok(Parser::expr_to_expr(token)),
                    _ => Err(ParseError::new(
                        format!("Expected value, got {}", token),
                        pos,
                        &self.lexer,
                    )),
                })
        })
    }

    fn discard_indent(&mut self) -> Result<(), ParseError> {
        self.handle_eof_and_comments("indent").and_then(|res| {
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
        self.handle_eof_and_comments("newline").and_then(|res| {
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
        self.handle_eof_and_comments("=").and_then(|res| {
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

    fn read_assignment(&mut self) -> Result<(&'a [u8], Expr<'a>), ParseError> {
        let var = self.expect_identifier_eating_indent()?;
        self.discard_assignment()?;
        let value = self.expect_value()?;
        Ok((var.value(), value))
    }

    // really need a peekable overlay while allowing us to access the lexer whenever we want
    // (mostly for errors).
    fn parse_rule(&mut self) -> Result<Rule<'a>, ParseError> {
        let identifier = self.expect_identifier()?;
        self.discard_newline()?;

        let mut bindings = HashMap::new();
        let mut at_least_one = false;
        loop {
            let item = self.peeker.peek(&mut self.lexer);
            if item.is_none() {
                if at_least_one {
                    break;
                } else {
                    return Err(ParseError::eof(
                        format!("Expected indent, got EOF"),
                        &self.lexer,
                    ));
                }
            }

            let item = item.unwrap();
            eprintln!("Continuing loop with {:?}", item);
            if let Ok((lexeme, _)) = &item {
                match lexeme {
                    Lexeme::Newline | Lexeme::Comment(_) => {
                        self.peeker.next(&mut self.lexer);
                        // continue looping.
                    }
                    Lexeme::Indent => {
                        // is an indent, do the rest of this loop.
                        at_least_one = true;
                        self.discard_indent()?;
                        let (var, value) = self.read_assignment()?;
                        // TODO: Move this to a semantic pass.
                        if !allowed_rule_variable(var) {
                            return Err(ParseError::new(
                                format!(
                                    "unexpected variable '{}'",
                                    std::str::from_utf8(var).unwrap_or("invalid utf-8")
                                ),
                                self.lexer.current_pos(),
                                &self.lexer,
                            ));
                        }
                        bindings.insert(var, value);
                    }
                    _ => {
                        // Done with this rule since we encountered a non-indent.
                        break;
                    }
                }
            }
        }

        Ok(Rule {
            name: identifier.value(),
            bindings,
        })
    }

    fn parse_build(&mut self) -> Result<Build<'a>, ParseError> {
        // TODO: Support all kinds of optional outputs and dependencies.
        #[derive(Debug, PartialEq, Eq)]
        enum Read {
            Outputs,
            Rule,
            Inputs,
        };

        let mut outputs: Vec<Expr<'a>> = Vec::new();
        let mut inputs: Vec<Expr<'a>> = Vec::new();
        let mut rule = None;
        let mut state = Read::Outputs;
        let mut first_line_pos = None;
        while let Some(result) = self.peeker.next(&mut self.lexer) {
            let (token, pos) =
                result.map_err(|lex_err| ParseError::from_lexer_error(lex_err, &self.lexer))?;
            if first_line_pos.is_none() {
                first_line_pos = Some(pos);
            }
            match state {
                Read::Outputs => match token {
                    Lexeme::Expr(_) => {
                        outputs.push(Parser::expr_to_expr(token));
                    }
                    Lexeme::Colon => {
                        if outputs.is_empty() {
                            return Err(ParseError::new(
                                "Expected at least one output for build",
                                pos,
                                &self.lexer,
                            ));
                        }
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
                    Lexeme::Expr(_) => {
                        inputs.push(Parser::expr_to_expr(token));
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
            bindings: Rc::new(RefCell::new(Env::default())),
            rules: Vec::new(),
            builds: Vec::new(),
            includes: Vec::new(),
        };
        // Focus here on handling bindings at the top-level, in rules and in builds.
        while let Some(result) = self.peeker.next(&mut self.lexer) {
            let (token, pos) =
                result.map_err(|lex_err| ParseError::from_lexer_error(lex_err, &self.lexer))?;
            match token {
                Lexeme::Identifier(ident) => {
                    self.discard_assignment()?;
                    let value = self.expect_value()?;
                    // Top-level bindings are evaluated immediately.
                    let value = {
                        let b = description.bindings.borrow();
                        value.eval(&b)
                    };
                    description.bindings.borrow_mut().add_binding(ident, value);
                }
                Lexeme::Rule => {
                    description.rules.push(self.parse_rule()?);
                }
                Lexeme::Build => {
                    description.builds.push(self.parse_build()?);
                }
                Lexeme::Include => {
                    let path = self.expect_value()?;
                    description.includes.push(Include { path });
                }
                Lexeme::Newline => {}
                Lexeme::Comment(_) => {}
                _ => {
                    return Err(ParseError::new(
                        format!("Unhandled token {:?}", token),
                        pos,
                        &self.lexer,
                    ));
                }
            }
        }
        Ok(description)
    }
}

const ALLOWED_RULE_VARIABLES: &[&[u8]] = &[b"command", b"description"];

fn allowed_rule_variable(name: &[u8]) -> bool {
    ALLOWED_RULE_VARIABLES.contains(&name)
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
                8,
                "=",
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
                "value",
            ),
            (
                r#"rule cc
  command="#,
                11,
                "value",
            ),
            (
                r#"rule cc
  command=
"#,
                11,
                "value",
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
