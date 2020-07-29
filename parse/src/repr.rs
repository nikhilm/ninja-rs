// Paths are canonicalized and mapped to a cache
// Rules are interned into indices.
// This actually needs to come after the variable evaluation pass.
#[derive(Debug, Default)]
pub struct Description {
    // will have things like pools and minimum ninja version and defaults and so on.
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
    pub implicit_inputs: Vec<Vec<u8>>,
    pub order_inputs: Vec<Vec<u8>>,
    pub outputs: Vec<Vec<u8>>,
}
