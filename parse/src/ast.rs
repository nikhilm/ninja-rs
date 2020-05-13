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
    pub(crate) rules: Vec<Rule<'a>>,
    pub(crate) builds: Vec<Build<'a>>,
    // defaults: _,
    // includes: _,
    // subninjas: _,
    // pools: _,
}

#[derive(Debug)]
pub struct Rule<'a> {
    pub(crate) name: BytesRef<'a>,
    pub(crate) command: BytesRef<'a>,
}

#[derive(Debug)]
pub struct Build<'a> {
    pub(crate) rule: BytesRef<'a>,
    // These will become structs once we discriminate inputs and outputs.
    pub(crate) inputs: Vec<BytesRef<'a>>,
    pub(crate) outputs: Vec<BytesRef<'a>>,
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
