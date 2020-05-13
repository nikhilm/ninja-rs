use string_interner::{DefaultStringInterner, Sym};

use ninja_parse::ast as past;
use thiserror::Error;

mod ast;
use ast::*;
use std::{
    collections::{hash_map::Entry, HashMap},
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

fn canonicalize(past: past::Description) -> Result<Description, ProcessingError> {
    // Using an interner that could accept bytes would allow us to not have to convert.
    let mut rules: HashMap<&[u8], past::Rule> = HashMap::with_capacity(past.rules.len());
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
    let mut paths = DefaultStringInterner::new();
    for build in &past.builds {
        for output in &build.outputs {
            let name = std::str::from_utf8(output)?;
            if let Some(_) = paths.get(name) {
                // TODO: Also add line/col information from token position, which isn't being preserved
                // right now!
                return Err(ProcessingError::DuplicateOutput(name.to_owned()));
            }
            paths.get_or_intern(name);
        }
    }

    let mut builds = Vec::with_capacity(past.builds.len());
    for build in past.builds {
        let rule = rules.get(build.rule);
        if let None = rule {
            return Err(ProcessingError::UnknownRule(
                std::str::from_utf8(build.rule)?.to_owned(),
            ));
        }
        let rule = rule.unwrap();
        builds.push(Build {
            action: match rule.name {
                [112, 104, 111, 110, 121] => Action::Phony,
                _ => Action::Command(std::str::from_utf8(rule.name)?.to_owned()),
            },
            inputs: build
                .inputs
                .into_iter()
                .map(|p| std::str::from_utf8(p).map(|s| paths.get_or_intern(s)))
                .collect::<Result<Vec<Sym>, Utf8Error>>()?,
            outputs: build
                .outputs
                .into_iter()
                .map(|p| std::str::from_utf8(p).map(|s| paths.get_or_intern(s)))
                .collect::<Result<Vec<Sym>, Utf8Error>>()?,
        })
    }

    Ok(Description { paths, builds })
}

pub fn to_description(past: past::Description) -> Result<Description, ProcessingError> {
    // Passes.
    // This should handle.
    // 1. TODO evaluating all variables to final values.
    // let ast = evaluate(ast);
    // 2. canonicalizing paths.
    let canonical = canonicalize(past)?;
    eprintln!("YO {:?}", canonical);
    Ok(canonical)
}

#[cfg(test)]
mod tests {
    #[test]
    fn write_me() {
        todo!();
    }
}
