// Holding place until we figure out refactor.
use ninja_metrics::scoped_metric;
use ninja_parse::{ast as past, Env, ParseError, Parser};
use std::{
    collections::{hash_map::Entry, HashMap, HashSet},
    path::Path,
    str::Utf8Error,
    string::FromUtf8Error,
};
use thiserror::Error;

pub trait Loader {
    type Error;
    fn load(&mut self, from: &[u8], request: &[u8]) -> Result<Vec<u8>, Self::Error>;
}

pub mod repr;
pub use repr::*;

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
    #[error("missing 'command' for rule: {0}")]
    MissingCommand(String),
    #[error("{0}")]
    ParseFailed(#[from] ParseError),
    #[error("{0}")]
    IoError(#[from] std::io::Error),
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
            bindings: HashMap::default(),
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
        // TODO: Use the environment in scope + the rule environment.
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
        // TODO: Get environment from rule!
        let mut env = Env::with_parent(past.bindings.clone());
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

                    let rule = rule.unwrap();
                    let command = rule.bindings.get("command".as_bytes());
                    if command.is_none() {
                        return Err(ProcessingError::MissingCommand(
                            std::str::from_utf8(rule.name)?.to_owned(),
                        ));
                    }

                    Action::Command(String::from_utf8(
                        command.unwrap().eval_for_build(&env, &rule),
                    )?)
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

fn to_description(
    loader: &mut dyn Loader,
    past: past::Description,
) -> Result<Description, ProcessingError> {
    // Passes.
    // This should handle.
    // 1. TODO evaluating all variables to final values.
    // let ast = evaluate(ast);
    // 2. canonicalizing paths.
    // 3. How does this handle include vs subninja and combining rules/edges with relevant
    //    namespacing.
    Ok(canonicalize(past)?)
}

pub fn build_representation(
    loader: &mut dyn Loader,
    start: &[u8],
) -> Result<Description, ProcessingError> {
    let contents = loader.load(start.as_ref())?;
    let ast = {
        scoped_metric!("parse");
        Parser::new(&contents, Some(start)).parse()?
    };
    {
        scoped_metric!("analyze");
        to_description(loader, ast)
    }
}

#[cfg(test)]
mod test {
    use insta::assert_debug_snapshot;

    use super::{Loader, ProcessingError};
    use ninja_parse::{ast as past, env::Env};
    use std::{cell::RefCell, rc::Rc};

    struct DummyLoader {}
    impl Loader for DummyLoader {
        fn load(&mut self, path: &std::path::Path) -> std::io::Result<Vec<u8>> {
            unimplemented!();
        }
    }
    // Small wrapper to supply a dummy loader when we know no includes are present.
    fn to_description(ast: past::Description) -> Result<Description, ProcessingError> {
        let mut loader = DummyLoader {};
        super::to_description(&mut loader, ast);
    }

    macro_rules! rule {
        ($name:literal) => {
            past::Rule {
                name: $name.as_bytes(),
                bindings: vec![(
                    "command".as_bytes(),
                    past::Expr(vec![past::Term::Literal(b"")]),
                )]
                .into_iter()
                .collect(),
            }
        };
        ($name:literal, $command:literal) => {
            past::Rule {
                name: $name.as_bytes(),
                bindings: vec![(
                    "command".as_bytes(),
                    past::Expr(vec![past::Term::Literal($command.as_bytes())]),
                )]
                .into_iter()
                .collect(),
            }
        };
    }

    #[test]
    fn no_rule_named_phony() {
        let desc = past::Description {
            includes: vec![],
            bindings: Rc::new(RefCell::new(Env::default())),
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
            includes: vec![],
            bindings: Rc::new(RefCell::new(Env::default())),
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
            includes: vec![],
            bindings: Rc::new(RefCell::new(Env::default())),
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
            includes: vec![],
            bindings: Rc::new(RefCell::new(Env::default())),
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
            includes: vec![],
            bindings: Rc::new(RefCell::new(Env::default())),
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
            includes: vec![],
            bindings: Rc::new(RefCell::new(Env::default())),
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
            includes: vec![],
            bindings: Rc::new(RefCell::new(Env::default())),
            rules: vec![past::Rule {
                name: b"echo",
                bindings: vec![(
                    "command".as_bytes(),
                    past::Expr(vec![
                        past::Term::Literal(b"echo "),
                        past::Term::Reference(b"in"),
                        past::Term::Literal(b" makes "),
                        past::Term::Reference(b"out"),
                    ]),
                )]
                .into_iter()
                .collect(),
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
