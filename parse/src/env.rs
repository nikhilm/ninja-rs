use std::collections::HashMap;

#[derive(Debug)]
enum Value {
    Bytes(Vec<u8>),
    SpaceSeparated(Vec<Vec<u8>>),
}

#[derive(Default)]
pub struct Env {
    // TODO: Switch to Vec<u8> instead of string.
    bindings: HashMap<String, Value>,
}

impl Env {
    pub fn add_binding(&mut self, name: String, value: Vec<Vec<u8>>) {
        self.bindings.insert(name, Value::SpaceSeparated(value));
    }

    pub fn lookup(&self, name: &str) -> Option<Vec<u8>> {
        let value = self.bindings.get(name)?;
        match value {
            Value::Bytes(v) => Some(v.clone()),
            Value::SpaceSeparated(v) => {
                let mut vec = Vec::new();
                for (i, el) in v.iter().enumerate() {
                    vec.extend(el);
                    if i != v.len() - 1 {
                        vec.push(b' ');
                    }
                }
                Some(vec)
            }
        }
    }
}
