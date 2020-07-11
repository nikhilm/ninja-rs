use ninja_parse::{ast as past, Env};
use std::{
    collections::{hash_map::Entry, HashMap, HashSet},
    str::Utf8Error,
    string::FromUtf8Error,
};
use thiserror::Error;

pub mod ast;
pub use ast::*;

#[derive(Error, Debug)]
#[error("some processing error TODO")]
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

fn canonicalize(past: past::Description) -> Result<Description, ProcessingError> {
    // Using an interner that could accept bytes would allow us to not have to convert.
    let mut rules: HashMap<&[u8], past::Rule> = HashMap::with_capacity(past.rules.len());
    // Insert built-in rules.
    rules.insert(
        PHONY,
        past::Rule {
            name: PHONY,
            command: past::Expr(vec![]),
        },
    );
    for rule in past.rules {
        match rules.entry(rule.name) {
            Entry::Occupied(_) => {
                // TODO: Also add line/col information from token position, which isn't being preserved
                // right now!
                return Err(ProcessingError::DuplicateRule(
                    std::str::from_utf8(rule.name)?.to_owned(),
                ));
            }
            Entry::Vacant(e) => {
                e.insert(rule);
            }
        }
    }

    let mut outputs_seen = HashSet::new();
    let mut builds = Vec::with_capacity(past.builds.len());
    for build in past.builds {
        let mut evaluated_outputs = Vec::with_capacity(build.outputs.len());
        // TODO: Use the environment in scope.
        let empty_env = Env::default();

        for output in &build.outputs {
            let output = output.eval(&empty_env);
            if outputs_seen.contains(&output) {
                // TODO: Also add line/col information from token position, which isn't being preserved
                // right now!
                return Err(ProcessingError::DuplicateOutput(
                    String::from_utf8(output)?.to_owned(),
                ));
            }
            outputs_seen.insert(output.clone());
            evaluated_outputs.push(output);
        }

        let evaluated_inputs: Vec<Vec<u8>> =
            build.inputs.iter().map(|i| i.eval(&empty_env)).collect();

        // TODO: Note that any rule/build level binding can refer to these variables, so the entire
        // build statement evaluation must have this environment available. In addition, these are
        // "shell quoted" when expanding within a command.
        let mut env = Env::default();
        env.add_binding(b"out".to_vec(), space_seperated_paths(&evaluated_outputs));
        env.add_binding(b"in".to_vec(), space_seperated_paths(&evaluated_inputs));

        let action = {
            match build.rule {
                [112, 104, 111, 110, 121] => Action::Phony,
                other => {
                    let rule = rules.get(other);
                    if rule.is_none() {
                        return Err(ProcessingError::UnknownRule(
                            std::str::from_utf8(other)?.to_owned(),
                        ));
                    }
                    Action::Command(String::from_utf8(rule.unwrap().command.eval(&env))?)
                }
            }
        };
        builds.push(Build {
            action,
            inputs: evaluated_inputs,
            outputs: evaluated_outputs,
        })
    }

    Ok(Description { builds })
}

pub fn to_description(past: past::Description) -> Result<Description, ProcessingError> {
    // Passes.
    // This should handle.
    // 1. TODO evaluating all variables to final values.
    // let ast = evaluate(ast);
    // 2. canonicalizing paths.
    Ok(canonicalize(past)?)
}

#[cfg(test)]
mod test {
    use insta::assert_debug_snapshot;

    use super::{to_description, ProcessingError};
    use ninja_parse::ast as past;

    macro_rules! rule {
        ($name:literal) => {
            past::Rule {
                name: $name.as_bytes(),
                command: past::Expr(vec![past::Term::Literal(b"")]),
            }
        };
        ($name:literal, $command:literal) => {
            past::Rule {
                name: $name.as_bytes(),
                command: past::Expr(vec![past::Term::Literal($command.as_bytes())]),
            }
        };
    }

    #[test]
    fn no_rule_named_phony() {
        let desc = past::Description {
            rules: vec![rule!["phony"]],
            builds: vec![],
        };
        let result = to_description(desc);
        let err = result.unwrap_err();
        assert!(matches!(err, ProcessingError::DuplicateRule(_)));
    }

    #[test]
    fn err_duplicate_rule() {
        let desc = past::Description {
            rules: vec![
                rule!("link", "link.exe"),
                rule!("compile", "compile.exe"),
                rule!("link", "link.exe"),
            ],
            builds: vec![],
        };
        let err = to_description(desc).unwrap_err();
        assert!(matches!(err, ProcessingError::DuplicateRule(_)));
    }

    #[test]
    fn duplicate_output() {
        let desc = past::Description {
            rules: vec![],
            builds: vec![
                past::Build {
                    rule: b"phony",
                    inputs: vec![],
                    outputs: vec![past::Expr(vec![past::Term::Literal(b"a.txt")])],
                },
                past::Build {
                    rule: b"phony",
                    inputs: vec![],
                    outputs: vec![past::Expr(vec![past::Term::Literal(b"a.txt")])],
                },
            ],
        };
        assert!(matches!(
            to_description(desc).unwrap_err(),
            ProcessingError::DuplicateOutput(_)
        ));
    }

    #[test]
    fn duplicate_output2() {
        let desc = past::Description {
            rules: vec![],
            builds: vec![
                past::Build {
                    rule: b"phony",
                    inputs: vec![],
                    outputs: vec![
                        past::Expr(vec![past::Term::Literal(b"b.txt")]),
                        past::Expr(vec![past::Term::Literal(b"a.txt")]),
                    ],
                },
                past::Build {
                    rule: b"phony",
                    inputs: vec![],
                    outputs: vec![
                        past::Expr(vec![past::Term::Literal(b"a.txt")]),
                        past::Expr(vec![past::Term::Literal(b"c.txt")]),
                    ],
                },
            ],
        };
        assert!(matches!(
            to_description(desc).unwrap_err(),
            ProcessingError::DuplicateOutput(_)
        ));
    }

    #[test]
    fn unknown_rule() {
        let desc = past::Description {
            rules: vec![],
            builds: vec![past::Build {
                rule: b"baloney",
                inputs: vec![],
                outputs: vec![past::Expr(vec![past::Term::Literal(b"a.txt")])],
            }],
        };
        assert!(matches!(
            to_description(desc).unwrap_err(),
            ProcessingError::UnknownRule(_)
        ));
    }

    #[test]
    fn success() {
        let desc = past::Description {
            rules: vec![
                rule!["link", "link.exe"],
                rule!["cc", "clang"],
                rule!["unused"],
            ],
            builds: vec![
                past::Build {
                    rule: b"phony",
                    inputs: vec![past::Expr(vec![past::Term::Literal(b"source.txt")])],
                    outputs: vec![past::Expr(vec![past::Term::Literal(b"a.txt")])],
                },
                past::Build {
                    rule: b"cc",
                    inputs: vec![
                        past::Expr(vec![past::Term::Literal(b"hello.c")]),
                        past::Expr(vec![past::Term::Literal(b"hello.h")]),
                    ],
                    outputs: vec![past::Expr(vec![past::Term::Literal(b"hello.o")])],
                },
                past::Build {
                    rule: b"link",
                    inputs: vec![
                        past::Expr(vec![past::Term::Literal(b"hello.o")]),
                        past::Expr(vec![past::Term::Literal(b"my_shared_lib.so")]),
                    ],
                    outputs: vec![past::Expr(vec![past::Term::Literal(b"hello")])],
                },
            ],
        };
        let ast = to_description(desc).unwrap();
        assert_debug_snapshot!(ast);
    }

    #[test]
    fn in_and_out_basic() {
        let ast = past::Description {
            rules: vec![past::Rule {
                name: b"echo",
                command: past::Expr(vec![
                    past::Term::Literal(b"echo "),
                    past::Term::Reference(b"in"),
                    past::Term::Literal(b" makes "),
                    past::Term::Reference(b"out"),
                ]),
            }],
            builds: vec![past::Build {
                rule: b"echo",
                inputs: vec![
                    past::Expr(vec![past::Term::Literal(b"a.txt")]),
                    past::Expr(vec![past::Term::Literal(b"b.txt")]),
                ],
                outputs: vec![
                    past::Expr(vec![past::Term::Literal(b"c.txt")]),
                    past::Expr(vec![past::Term::Literal(b"d.txt")]),
                ],
            }],
        };
        let ast = to_description(ast).unwrap();
        assert_debug_snapshot!(ast);
    }
}
