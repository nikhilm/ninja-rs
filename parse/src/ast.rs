use crate::env::Env;
use std::{cell::RefCell, rc::Rc};

pub type BytesRef<'a> = &'a [u8];

/*
struct Binding {
    name: _,
    expr: _,
}
*/

#[derive(Debug)]
pub struct Description<'a> {
    pub bindings: Rc<RefCell<Env>>,
    pub rules: Vec<Rule<'a>>,
    pub builds: Vec<Build<'a>>,
    // defaults: _,
    // includes: _,
    // subninjas: _,
    // pools: _,
}

#[derive(Debug)]
pub enum Term<'a> {
    Literal(BytesRef<'a>),
    Reference(BytesRef<'a>),
}

#[derive(Debug)]
pub struct Expr<'a>(pub Vec<Term<'a>>);

impl<'a> Expr<'a> {
    pub fn eval(&self, env: &Env) -> Vec<u8> {
        let mut result = Vec::new();
        for term in &self.0 {
            match term {
                Term::Literal(bytes) => result.extend_from_slice(bytes),
                Term::Reference(name) => {
                    result.extend(env.lookup(*name).unwrap_or_default());
                }
            }
        }
        result
    }
}

#[derive(Debug)]
pub struct Rule<'a> {
    pub name: BytesRef<'a>,
    pub command: Expr<'a>,
}

#[derive(Debug)]
pub struct Build<'a> {
    pub rule: BytesRef<'a>,
    // These will become structs once we discriminate inputs and outputs.
    pub inputs: Vec<Expr<'a>>,
    pub outputs: Vec<Expr<'a>>,
    // ...
}

/*
struct Default {
    targets: _,
}

struct Include {
    path: _,
}

struct Subninja {
    path: _,
}
*/
