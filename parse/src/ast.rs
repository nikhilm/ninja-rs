use super::env::Env;
use std::collections::HashMap;

#[derive(Debug)]
pub enum Term {
    Literal(Vec<u8>),
    Reference(Vec<u8>),
}

#[derive(Debug)]
pub struct Expr(pub Vec<Term>);

impl Expr {
    pub fn eval(&self, env: &Env) -> Vec<u8> {
        let mut result = Vec::new();
        for term in &self.0 {
            match term {
                Term::Literal(bytes) => result.extend_from_slice(bytes),
                Term::Reference(name) => {
                    result.extend(env.lookup(name.as_slice()).unwrap_or_default());
                }
            }
        }
        result
    }

    pub fn eval_for_build(&self, env: &Env, rule: &Rule) -> Vec<u8> {
        let mut result = Vec::new();
        for term in &self.0 {
            match term {
                Term::Literal(bytes) => result.extend_from_slice(bytes),
                Term::Reference(name) => {
                    result.extend(
                        env.lookup_for_build(rule, name.as_slice())
                            .unwrap_or_default(),
                    );
                }
            }
        }
        result
    }
}

#[derive(Debug)]
pub struct Rule {
    pub name: Vec<u8>,
    pub bindings: HashMap<Vec<u8>, Expr>,
}

#[derive(Debug)]
pub struct Build {
    pub rule: Vec<u8>,
    // These will become structs once we discriminate inputs and outputs.
    pub inputs: Vec<Expr>,
    pub outputs: Vec<Expr>,
    // ...
}
