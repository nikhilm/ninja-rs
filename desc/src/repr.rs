// Paths are canonicalized and mapped to a cache
// Rules are interned into indices.
// This actually needs to come after the variable evaluation pass.
#[derive(Debug)]
pub struct Description {
    pub builds: Vec<Build>,
}

#[derive(Debug)]
pub enum Action {
    Phony,
    Command(String),
}

#[derive(Debug)]
pub struct Build {
    pub action: Action,
    pub inputs: Vec<Vec<u8>>,
    pub outputs: Vec<Vec<u8>>,
}
