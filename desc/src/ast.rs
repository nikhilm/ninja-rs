use string_interner::{DefaultStringInterner, Sym};
// Paths are canonicalized and mapped to a cache
// Rules are interned into indices.
// This actually needs to come after the variable evaluation pass.
#[derive(Debug)]
pub struct Description {
    pub(crate) paths: DefaultStringInterner,
    pub(crate) builds: Vec<Build>,
}

#[derive(Debug)]
pub enum Action {
    Phony,
    Command(String),
}

#[derive(Debug)]
pub struct Build {
    pub(crate) action: Action,
    pub(crate) inputs: Vec<Sym>,
    pub(crate) outputs: Vec<Sym>,
}
