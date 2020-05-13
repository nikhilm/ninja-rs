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
pub struct Rule<'a> {
    pub name: BytesRef<'a>,
    pub command: BytesRef<'a>,
}

#[derive(Debug)]
pub struct Build<'a> {
    pub rule: BytesRef<'a>,
    // These will become structs once we discriminate inputs and outputs.
    pub inputs: Vec<BytesRef<'a>>,
    pub outputs: Vec<BytesRef<'a>>,
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
