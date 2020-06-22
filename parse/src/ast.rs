pub type BytesRef<'a> = &'a [u8];

/*
struct Binding {
    name: _,
    expr: _,
}
*/

#[derive(Debug)]
pub struct Description<'a> {
    // bindings: _,
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
    // Will eventually accept an environment. Plus possibly re-use some of these "strings"
    pub fn eval(&self) -> Vec<u8> {
        let mut result = Vec::new();
        for term in &self.0 {
            match term {
                Term::Literal(bytes) => result.extend_from_slice(bytes),
                Term::Reference(_) => todo!("Don't know how"),
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
