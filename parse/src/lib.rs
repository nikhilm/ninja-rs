/*
 * Copyright 2020 Nikhil Marathe <nsm.nikhil@gmail.com>
 *
 * Licensed under the Apache License, Version 2.0 (the "License");
 * you may not use this file except in compliance with the License.
 * You may obtain a copy of the License at
 *
 *     http://www.apache.org/licenses/LICENSE-2.0
 *
 * Unless required by applicable law or agreed to in writing, software
 * distributed under the License is distributed on an "AS IS" BASIS,
 * WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
 * See the License for the specific language governing permissions and
 * limitations under the License.
 */

#![feature(is_sorted)]
// Holding place until we figure out refactor.
use ast as past;
use ninja_metrics::scoped_metric;
use std::{
    cell::RefCell,
    collections::{HashMap, HashSet},
    rc::Rc,
    str::Utf8Error,
    string::FromUtf8Error,
};
use thiserror::Error;

pub trait Loader {
    fn load(&mut self, from: Option<&[u8]>, request: &[u8]) -> Result<Vec<u8>, std::io::Error>;
}

mod ast;
mod env;
mod lexer;
mod parser;
pub mod repr;

use env::Env;
use parser::{ParseError, Parser};
pub use repr::*;

#[derive(Error, Debug)]
#[error("{position}: {inner}")]
pub struct ProcessingErrorWithPosition {
    inner: ProcessingError,
    position: lexer::Position,
}

#[derive(Error, Debug)]
pub enum ProcessingError {
    #[error("utf-8 error")]
    Utf8Error(#[from] Utf8Error),
    #[error("string utf-8 error")]
    StringUtf8Error(#[from] FromUtf8Error),
    #[error("duplicate rule name: {0}")]
    DuplicateRule(String),
    #[error("duplicate output: {0}")]
    DuplicateOutput(String),
    #[error("build edge refers to unknown rule: {0}")]
    UnknownRule(String),
    #[error("missing 'command' for rule: {0}")]
    MissingCommand(String),
    #[error(transparent)]
    ParseFailed(#[from] ParseError),
    #[error(transparent)]
    IoError(#[from] std::io::Error),
    #[error(transparent)]
    WithPosition(#[from] Box<ProcessingErrorWithPosition>),
}

impl ProcessingError {
    fn with_position(self, position: lexer::Position) -> ProcessingErrorWithPosition {
        ProcessingErrorWithPosition {
            inner: self,
            position,
        }
    }

    fn with_position_boxed(self, position: lexer::Position) -> Box<ProcessingErrorWithPosition> {
        Box::new(self.with_position(position))
    }
}

const PHONY: &[u8] = &[112, 104, 111, 110, 121];

fn space_seperated_paths(paths: &Vec<Vec<u8>>) -> Vec<u8> {
    let mut vec = Vec::new();
    for (i, el) in paths.iter().enumerate() {
        vec.extend(el);
        if i != paths.len() - 1 {
            vec.push(b' ');
        }
    }
    vec
}

struct ParseState {
    known_rules: HashMap<Vec<u8>, past::Rule>,
    outputs_seen: HashSet<Vec<u8>>,
    description: Description,
    bindings: Rc<RefCell<Env>>,
}

impl Default for ParseState {
    fn default() -> Self {
        let mut rules = HashMap::default();
        // Insert built-in rules.
        rules.insert(
            PHONY.to_vec(),
            past::Rule {
                name: PHONY.to_vec(),
                bindings: HashMap::default(),
            },
        );
        Self {
            known_rules: rules,
            outputs_seen: HashSet::default(),
            description: Description::default(),
            bindings: Rc::new(RefCell::new(Env::default())),
        }
    }
}

impl ParseState {
    fn add_rule(&mut self, rule: past::Rule) -> Result<(), ProcessingError> {
        if self.known_rules.get(&rule.name).is_some() {
            // TODO: Also add line/col information from token position, which isn't being preserved
            // right now!
            Err(ProcessingError::DuplicateRule(
                std::str::from_utf8(&rule.name)?.to_owned(),
            ))
        } else {
            self.known_rules.insert(rule.name.clone(), rule);
            Ok(())
        }
    }

    fn add_build_edge(
        &mut self,
        build: past::Build,
        _top: Rc<RefCell<Env>>,
    ) -> Result<(), ProcessingError> {
        let mut evaluated_outputs = Vec::with_capacity(build.outputs.len());
        // TODO: Use the environment in scope + the rule environment.
        // TODO: Are the build bindings available to the input and output path evaluation?

        for output in &build.outputs {
            let output = output.eval(&build.bindings);
            if self.outputs_seen.contains(&output) {
                // TODO: Also add line/col information from token position, which isn't being preserved
                // right now!
                return Err(ProcessingError::DuplicateOutput(
                    String::from_utf8(output)?.to_owned(),
                ));
            }
            self.outputs_seen.insert(output.clone());
            evaluated_outputs.push(output);
        }

        let evaluated_inputs: Vec<Vec<u8>> = build
            .inputs
            .iter()
            .map(|i| i.eval(&build.bindings))
            .collect();

        let evaluated_implicit_inputs: Vec<Vec<u8>> = build
            .implicit_inputs
            .iter()
            .map(|i| i.eval(&build.bindings))
            .collect();

        let evaluated_order_inputs: Vec<Vec<u8>> = build
            .order_inputs
            .iter()
            .map(|i| i.eval(&build.bindings))
            .collect();

        // TODO: Note that any rule/build level binding can refer to these variables, so the entire
        // build statement evaluation must have this environment available. In addition, these are
        // "shell quoted" when expanding within a command.
        // TODO: Get environment from rule!
        let mut env = Env::with_parent(Rc::new(RefCell::new(build.bindings)));
        env.add_binding(b"out".to_vec(), space_seperated_paths(&evaluated_outputs));
        env.add_binding(b"in".to_vec(), space_seperated_paths(&evaluated_inputs));

        let action = {
            match build.rule.as_slice() {
                [112, 104, 111, 110, 121] => Action::Phony,
                other => {
                    let rule = self.known_rules.get(other);
                    if rule.is_none() {
                        return Err(ProcessingError::UnknownRule(
                            std::str::from_utf8(&other)?.to_owned(),
                        ));
                    }

                    let rule = rule.unwrap();
                    let command = rule.bindings.get("command".as_bytes());
                    if command.is_none() {
                        return Err(ProcessingError::MissingCommand(
                            std::str::from_utf8(&rule.name)?.to_owned(),
                        ));
                    }

                    Action::Command(String::from_utf8(
                        command.unwrap().eval_for_build(&env, &rule),
                    )?)
                }
            }
        };
        self.description.builds.push(Build {
            action,
            inputs: evaluated_inputs,
            implicit_inputs: evaluated_implicit_inputs,
            order_inputs: evaluated_order_inputs,
            outputs: evaluated_outputs,
        });
        Ok(())
    }

    fn add_default(&mut self, entries: Vec<u8>) {
        if self.description.defaults.is_none() {
            self.description.defaults = Some(HashSet::new());
        }
        self.description.defaults.as_mut().unwrap().insert(entries);
    }

    fn into_description(self) -> Description {
        self.description
    }
}

fn parse_single(
    contents: &[u8],
    name: Option<Vec<u8>>,
    state: &mut ParseState,
    loader: &mut dyn Loader,
) -> Result<(), ProcessingError> {
    Parser::new(&contents, name).parse(state, loader)?;
    Ok(())
}

pub fn build_representation(
    loader: &mut dyn Loader,
    start: Vec<u8>,
) -> Result<Description, ProcessingError> {
    scoped_metric!("parse");
    let mut state = ParseState::default();
    let contents = loader.load(None, &start)?;
    parse_single(&contents, Some(start), &mut state, loader)?;
    Ok(state.into_description())
}

#[cfg(test)]
mod test {

    use super::{ast as past, ParseState, ProcessingError};
    use crate::env::Env;
    use insta::assert_debug_snapshot;
    use std::{cell::RefCell, rc::Rc};

    macro_rules! lit {
        ($name:expr) => {
            past::Term::Literal($name.to_vec())
        };
    }

    macro_rules! aref {
        ($name:literal) => {
            past::Term::Reference($name.to_vec())
        };
    }

    macro_rules! rule {
        ($name:literal) => {
            past::Rule {
                name: $name.as_bytes().to_vec(),
                bindings: vec![(b"command".to_vec(), past::Expr(vec![lit!(b"")]))]
                    .into_iter()
                    .collect(),
            }
        };
        ($name:literal, $command:literal) => {
            past::Rule {
                name: $name.as_bytes().to_vec(),
                bindings: vec![(
                    b"command".to_vec(),
                    past::Expr(vec![lit!($command.as_bytes())]),
                )]
                .into_iter()
                .collect(),
            }
        };
    }

    #[test]
    fn no_rule_named_phony() {
        let mut parse_state = ParseState::default();
        let err = parse_state.add_rule(rule!["phony"]).unwrap_err();
        assert!(matches!(err, ProcessingError::DuplicateRule(_)));
    }

    #[test]
    fn err_duplicate_rule() {
        let mut parse_state = ParseState::default();
        let _ = parse_state.add_rule(rule!["link"]).unwrap();
        let _ = parse_state.add_rule(rule!["compile"]).unwrap();
        let err = parse_state.add_rule(rule!["link"]).expect_err("duplicate");
        assert!(matches!(err, ProcessingError::DuplicateRule(_)));
    }

    #[test]
    fn duplicate_output() {
        let mut parse_state = ParseState::default();
        let env = Rc::new(RefCell::new(Env::default()));
        let _ = parse_state
            .add_build_edge(
                past::Build {
                    rule: b"phony".to_vec(),
                    outputs: vec![past::Expr(vec![lit!(b"a.txt")])],
                    ..Default::default()
                },
                env.clone(),
            )
            .unwrap();
        let err = parse_state
            .add_build_edge(
                past::Build {
                    rule: b"phony".to_vec(),
                    outputs: vec![past::Expr(vec![lit!(b"a.txt")])],
                    ..Default::default()
                },
                env.clone(),
            )
            .expect_err("duplicate output");
        assert!(matches!(err, ProcessingError::DuplicateOutput(_)));
    }

    #[test]
    fn duplicate_output2() {
        let mut parse_state = ParseState::default();
        let env = Rc::new(RefCell::new(Env::default()));
        let _ = parse_state
            .add_build_edge(
                past::Build {
                    rule: b"phony".to_vec(),
                    outputs: vec![
                        past::Expr(vec![lit!(b"b.txt")]),
                        past::Expr(vec![lit!(b"a.txt")]),
                    ],
                    ..Default::default()
                },
                env.clone(),
            )
            .unwrap();
        let err = parse_state
            .add_build_edge(
                past::Build {
                    rule: b"phony".to_vec(),
                    outputs: vec![
                        past::Expr(vec![lit!(b"a.txt")]),
                        past::Expr(vec![lit!(b"c.txt")]),
                    ],
                    ..Default::default()
                },
                env.clone(),
            )
            .expect_err("duplicate output");
        assert!(matches!(err, ProcessingError::DuplicateOutput(_)));
    }

    #[test]
    fn unknown_rule() {
        let mut parse_state = ParseState::default();
        let env = Rc::new(RefCell::new(Env::default()));
        let err = parse_state
            .add_build_edge(
                past::Build {
                    rule: b"baloney".to_vec(),
                    ..Default::default()
                },
                env,
            )
            .expect_err("unknown rule");
        assert!(matches!(err, ProcessingError::UnknownRule(_)));
    }

    #[test]
    fn success() {
        let mut parse_state = ParseState::default();
        let env = Rc::new(RefCell::new(Env::default()));

        for rule in vec![
            rule!["link", "link.exe"],
            rule!["cc", "clang"],
            rule!["unused"],
        ] {
            parse_state.add_rule(rule).unwrap();
        }

        for build in vec![
            past::Build {
                rule: b"phony".to_vec(),
                inputs: vec![past::Expr(vec![lit!(b"source.txt")])],
                outputs: vec![past::Expr(vec![lit!(b"a.txt")])],
                ..Default::default()
            },
            past::Build {
                rule: b"cc".to_vec(),
                inputs: vec![
                    past::Expr(vec![lit!(b"hello.c")]),
                    past::Expr(vec![lit!(b"hello.h")]),
                ],
                outputs: vec![past::Expr(vec![lit!(b"hello.o")])],
                ..Default::default()
            },
            past::Build {
                rule: b"link".to_vec(),
                inputs: vec![
                    past::Expr(vec![lit!(b"hello.o")]),
                    past::Expr(vec![lit!(b"my_shared_lib.so")]),
                ],
                outputs: vec![past::Expr(vec![lit!(b"hello")])],
                ..Default::default()
            },
        ] {
            parse_state.add_build_edge(build, env.clone()).unwrap();
        }
        let repr = parse_state.into_description();
        assert_debug_snapshot!(repr);
    }

    #[test]
    fn in_and_out_basic() {
        let mut parse_state = ParseState::default();
        let env = Rc::new(RefCell::new(Env::default()));
        parse_state
            .add_rule(past::Rule {
                name: b"echo".to_vec(),
                bindings: vec![(
                    b"command".to_vec(),
                    past::Expr(vec![
                        lit!(b"echo "),
                        aref!(b"in"),
                        lit!(b" makes "),
                        aref!(b"out"),
                    ]),
                )]
                .into_iter()
                .collect(),
            })
            .unwrap();
        for build in vec![past::Build {
            rule: b"echo".to_vec(),
            inputs: vec![
                past::Expr(vec![lit!(b"a.txt")]),
                past::Expr(vec![lit!(b"b.txt")]),
            ],
            outputs: vec![
                past::Expr(vec![lit!(b"c.txt")]),
                past::Expr(vec![lit!(b"d.txt")]),
            ],
            ..Default::default()
        }] {
            let _ = parse_state.add_build_edge(build, env.clone()).unwrap();
        }
        let repr = parse_state.into_description();
        assert_debug_snapshot!(repr);
    }
}
