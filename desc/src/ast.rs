pub use string_interner::{DefaultStringInterner, Sym};
// Paths are canonicalized and mapped to a cache
// Rules are interned into indices.
// This actually needs to come after the variable evaluation pass.
#[derive(Debug)]
pub struct Description {
    pub(crate) paths: DefaultStringInterner,
    pub(crate) builds: Vec<Build>,
}

impl Description {
    pub fn consume(self) -> (DefaultStringInterner, Vec<Build>) {
        (self.paths, self.builds)
    }
}

#[derive(Debug)]
pub enum Action {
    Phony,
    Command(String),
}

#[derive(Debug)]
pub struct Build {
    pub action: Action,
    pub inputs: Vec<Sym>,
    pub outputs: Vec<Sym>,
}
