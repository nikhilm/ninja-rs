use ninja_parse::ast as past;
use thiserror::Error;

pub mod ast;
pub use ast::*;
use std::{
    collections::{hash_map::Entry, HashMap, HashSet},
    str::Utf8Error,
};

#[derive(Error, Debug)]
#[error("some processing error TODO")]
pub enum ProcessingError {
    #[error("utf-8 error")]
    Utf8Error(#[from] Utf8Error),
    #[error("duplicate rule name: {0}")]
    DuplicateRule(String),
    #[error("duplicate output: {0}")]
    DuplicateOutput(String),
    #[error("build edge refers to unknown rule: {0}")]
    UnknownRule(String),
}

const PHONY: &[u8] = &[112, 104, 111, 110, 121];

fn canonicalize(past: past::Description) -> Result<Description, ProcessingError> {
    // Using an interner that could accept bytes would allow us to not have to convert.
    let mut rules: HashMap<&[u8], past::Rule> = HashMap::with_capacity(past.rules.len());
    // Insert built-in rules.
    rules.insert(
        PHONY,
        past::Rule {
            name: PHONY,
            command: &[],
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

    // We process outputs first so duplicate detection can be done without confusing with inputs.
    let mut outputs_seen = HashSet::new();
    for build in &past.builds {
        for output in &build.outputs {
            if outputs_seen.contains(output) {
                // TODO: Also add line/col information from token position, which isn't being preserved
                // right now!
                return Err(ProcessingError::DuplicateOutput(
                    std::str::from_utf8(output)?.to_owned(),
                ));
            }
            outputs_seen.insert(output);
        }
    }

    let mut builds = Vec::with_capacity(past.builds.len());
    for build in past.builds {
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
                    Action::Command(std::str::from_utf8(rule.unwrap().command)?.to_owned())
                }
            }
        };
        builds.push(Build {
            action,
            inputs: build.inputs.iter().map(|i| i.to_vec()).collect(),
            outputs: build.outputs.iter().map(|i| i.to_vec()).collect(),
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
                command: b"",
            }
        };
        ($name:literal, $command:literal) => {
            past::Rule {
                name: $name.as_bytes(),
                command: $command.as_bytes(),
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
                    outputs: vec![b"a.txt"],
                },
                past::Build {
                    rule: b"phony",
                    inputs: vec![],
                    outputs: vec![b"a.txt"],
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
                    outputs: vec![b"b.txt", b"a.txt"],
                },
                past::Build {
                    rule: b"phony",
                    inputs: vec![],
                    outputs: vec![b"a.txt", b"c.txt"],
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
                outputs: vec![b"a.txt"],
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
                    inputs: vec![b"source.txt"],
                    outputs: vec![b"a.txt"],
                },
                past::Build {
                    rule: b"cc",
                    inputs: vec![b"hello.c", b"hello.h"],
                    outputs: vec![b"hello.o"],
                },
                past::Build {
                    rule: b"link",
                    inputs: vec![b"hello.o", b"my_shared_lib.so"],
                    outputs: vec![b"hello"],
                },
            ],
        };
        let ast = to_description(desc).unwrap();
        assert_debug_snapshot!(ast);
    }
}
