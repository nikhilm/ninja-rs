use std::collections::HashMap;

#[derive(Default)]
pub struct Env {
    bindings: HashMap<Vec<u8>, Vec<u8>>,
}

impl Env {
    pub fn add_binding<V1: Into<Vec<u8>>, V2: Into<Vec<u8>>>(&mut self, name: V1, value: V2) {
        self.bindings.insert(name.into(), value.into());
    }

    pub fn lookup<'a, V: Into<&'a [u8]>>(&self, name: V) -> Option<Vec<u8>> {
        Some(self.bindings.get(name.into())?.clone())
    }
}
