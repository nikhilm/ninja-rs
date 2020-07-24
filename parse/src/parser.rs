use std::{
    cell::RefCell,
    collections::HashMap,
    fmt::{Display, Formatter},
    rc::Rc,
};

use thiserror::Error;

use super::{
    ast::*,
    env::Env,
    lexer,
    lexer::{Lexeme, Lexer, LexerError, LexerItem, Position},
    Loader, ParseState, ProcessingError,
};

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
            "{source}:{lineno}:{col}: {msg}\n{line}\n{indent}^ near here",
            source = std::str::from_utf8(self.position.source_name.as_deref().unwrap_or(&[]))
                .unwrap_or("invalid utf-8"),
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
    source_name: Option<Vec<u8>>,
}

impl<'a> Parser<'a> {
    pub fn new(input: &[u8], source_name: Option<Vec<u8>>) -> Parser {
        Parser {
            lexer: Lexer::new(input, source_name.clone()),
            peeker: Default::default(),
            source_name,
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

    fn expr_to_expr(lexeme: Lexeme<'a>) -> Expr {
        lexeme.check();
        if let Lexeme::Expr(items) = lexeme {
            Expr(
                items
                    .iter()
                    .map(|item| match item {
                        Lexeme::Literal(v) | Lexeme::Escape(v) => Term::Literal(v.clone().to_vec()),
                        Lexeme::VarRef(_, v) => Term::Reference(v.clone().to_vec()),
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

    fn expect_value(&mut self) -> Result<Expr, ParseError> {
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

    fn read_assignment(&mut self) -> Result<(&'a [u8], Expr), ParseError> {
        let var = self.expect_identifier_eating_indent()?;
        self.discard_assignment()?;
        let value = self.expect_value()?;
        Ok((var.value(), value))
    }

    // really need a peekable overlay while allowing us to access the lexer whenever we want
    // (mostly for errors).
    fn parse_rule(&mut self) -> Result<Rule, ParseError> {
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
                        bindings.insert(var.to_vec(), value);
                    }
                    _ => {
                        // Done with this rule since we encountered a non-indent.
                        break;
                    }
                }
            }
        }

        Ok(Rule {
            name: identifier.value().to_vec(),
            bindings,
        })
    }

    fn parse_build(&mut self, top_env: Rc<RefCell<Env>>) -> Result<Build, ParseError> {
        // TODO: Support all kinds of optional outputs and dependencies.
        #[derive(Debug, PartialEq, Eq)]
        enum Read {
            Outputs,
            Rule,
            Inputs,
        };

        let mut outputs: Vec<Expr> = Vec::new();
        let mut inputs: Vec<Expr> = Vec::new();
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

        // EOF is OK as long as our state machine is done.
        if state != Read::Inputs {
            return Err(ParseError::eof(
                "unexpected EOF in the middle of a build edge",
                &self.lexer,
            ));
        }

        let mut edge = Build {
            rule: rule.take().unwrap().to_vec(),
            inputs,
            outputs,
            bindings: Env::with_parent(top_env),
        };

        loop {
            let item = self.peeker.peek(&mut self.lexer);
            if item.is_none() {
                break;
            }

            let item = item.unwrap();
            if let Ok((lexeme, _)) = &item {
                match lexeme {
                    Lexeme::Newline | Lexeme::Comment(_) => {
                        self.peeker.next(&mut self.lexer);
                        // continue looping.
                    }
                    Lexeme::Indent => {
                        // is an indent, do the rest of this loop.
                        self.discard_indent()?;
                        let (var, value) = self.read_assignment()?;
                        // TODO: Are bindings allowed to refer to:
                        // 1. $outs and $ins
                        // 2. bindings that come after them lexically but in the same edge
                        // Will need to use eval_for_build based on that.
                        edge.bindings.add_binding(var, value.eval(&edge.bindings));
                    }
                    _ => {
                        // Done with this rule since we encountered a non-indent.
                        break;
                    }
                }
            }
        }

        Ok(edge)
    }

    pub(crate) fn parse(
        mut self,
        state: &mut ParseState,
        loader: &mut dyn Loader,
    ) -> Result<(), ProcessingError> {
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
                        let b = state.bindings.borrow();
                        value.eval(&b)
                    };
                    state.bindings.borrow_mut().add_binding(ident, value);
                }
                Lexeme::Rule => {
                    state.add_rule(self.parse_rule()?)?;
                }
                Lexeme::Build => {
                    state.add_build_edge(
                        self.parse_build(state.bindings.clone())?,
                        state.bindings.clone(),
                    )?;
                }
                Lexeme::Include => {
                    let path = self.expect_value()?;
                    self.discard_newline()?;
                    let path = {
                        let env = state.bindings.borrow();
                        path.eval(&env)
                    };
                    let contents = loader.load(self.source_name.as_deref(), &path)?;
                    // TODO: Error should be from the included path.
                    super::parse_single(&contents, Some(path), state, loader)?;
                }
                Lexeme::Newline => {}
                Lexeme::Comment(_) => {}
                _ => {
                    return Err(ProcessingError::ParseFailed(ParseError::new(
                        format!("Unhandled token {:?}", token),
                        pos,
                        &self.lexer,
                    )));
                }
            }
        }
        Ok(())
    }
}

const ALLOWED_RULE_VARIABLES: &[&[u8]] = &[b"command", b"description"];

fn allowed_rule_variable(name: &[u8]) -> bool {
    ALLOWED_RULE_VARIABLES.contains(&name)
}

#[cfg(test)]
mod test {
    use super::super::{parse_single, Description, Loader, ParseState, ProcessingError};
    use insta::assert_debug_snapshot;

    struct DummyLoader {}

    impl Loader for DummyLoader {
        fn load(&mut self, _from: Option<&[u8]>, _load: &[u8]) -> std::io::Result<Vec<u8>> {
            unimplemented!();
        }
    }

    fn simple_parser(input: &[u8]) -> Result<Description, ProcessingError> {
        let mut parse_state = ParseState::default();
        let mut loader = DummyLoader {};
        let _ = parse_single(input, None, &mut parse_state, &mut loader)?;
        Ok(parse_state.into_description())
    }

    #[test]
    fn test_simple() {
        let input = r#"
rule cc
    command = gcc -c foo.c

build foo.o: cc foo.c"#;
        // TODO: The parser needs some mechanism to load other "files" when includes or subninjas
        // are encountered.
        let ast = simple_parser(input.as_bytes()).expect("valid parse");
        assert_debug_snapshot!(ast);
    }

    #[test]
    fn test_rule_identifier_fail() {
        for (input, expected_col) in &[("rule cc:", 8), ("rule", 5), ("rule\n", 5)] {
            let err = simple_parser(input.as_bytes()).unwrap_err();
            let err = match err {
                ProcessingError::ParseFailed(e) => e,
                e @ _ => panic!("Unexpected error {:?}", e),
            };
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
            let err = simple_parser(input.as_bytes()).unwrap_err();
            let err = match err {
                ProcessingError::ParseFailed(e) => e,
                e @ _ => panic!("Unexpected error {:?}", e),
            };
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
            let ast = simple_parser(with_rule.as_bytes()).expect("valid parse");
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
            let _ = simple_parser(input.as_bytes()).expect_err("parse should fail");
        }
    }
}
