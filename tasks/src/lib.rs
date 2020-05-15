use ninja_desc::ast::*;
use std::{collections::HashMap, fmt::Display};

#[derive(Debug, PartialOrd, Ord, Hash, Eq, PartialEq, Clone)]
pub enum Key {
    Single(Sym),
    Multi(Vec<Sym>),
}

#[derive(Debug)]
pub enum Task {
    // Indicates that this key just depends on another, usually Multi key.
    // Also used to map Phony.
    Retrieve,
    Command(String),
}

pub type Dependencies = Vec<Key>;
pub type TasksMap = HashMap<Key, Task>;

pub struct Tasks {
    paths: DefaultStringInterner,
    map: TasksMap,
    deps: HashMap<Key, Vec<Key>>,
}

impl Display for Tasks {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut write_key = |f: &mut std::fmt::Formatter<'_>, key: &Key| -> std::fmt::Result {
            match key {
                Key::Single(sym) => write!(f, "{}", self.paths.resolve(*sym).unwrap()),
                Key::Multi(syms) => write!(
                    f,
                    "{:?}",
                    syms.iter()
                        .map(|sym| self.paths.resolve(*sym).unwrap())
                        .collect::<Vec<&str>>()
                ),
            }
        };

        write!(f, "Tasks{{\n tasks:\n")?;
        let mut keys: Vec<&Key> = self.map.keys().collect();
        keys.sort();
        for key in keys {
            write!(f, "  ")?;
            write_key(f, key)?;
            write!(f, " -> {:?}\n", self.map[key])?;
        }

        write!(f, " deps:\n")?;
        let mut keys: Vec<&Key> = self.map.keys().collect();
        keys.sort();
        for key in keys {
            write!(f, "  ")?;
            write_key(f, key)?;
            write!(f, " -> ")?;
            for key in &self.deps[key] {
                write_key(f, key)?;
                write!(f, ", ")?;
            }
            write!(f, "\n")?;
        }
        write!(f, "}}")
    }
}

fn sym_to_key(output: Sym) -> Key {
    Key::Single(output)
}

fn syms_to_key(mut outputs: Vec<Sym>) -> Key {
    outputs.sort();
    Key::Multi(outputs)
}

pub fn description_to_tasks(desc: Description) -> Tasks {
    let (paths, builds) = desc.consume();
    let mut map: TasksMap = HashMap::new();
    let mut deps: HashMap<Key, Vec<Key>> = HashMap::new();
    // Since no two build edges can produce any single output, they also cannot produce any
    // multi-outputs. This means every build's outputs are guaranteed to be unique and we may as
    // well create a new key for each.
    for build in builds {
        let task = match build.action {
            Action::Phony => Task::Retrieve,
            Action::Command(s) => Task::Command(s),
        };
        let key = if build.outputs.len() == 1 {
            sym_to_key(build.outputs[0])
        } else {
            let main_key = syms_to_key(build.outputs);
            if let Key::Multi(ref syms) = main_key {
                for sym in syms {
                    let key = sym_to_key(*sym);
                    map.insert(key.clone(), Task::Retrieve);
                    // TODO: Stop cloning.
                    deps.insert(key, vec![main_key.clone()]);
                }
            } else {
                unreachable!();
            }
            main_key
        };
        map.insert(key.clone(), task);
        deps.insert(key, build.inputs.into_iter().map(sym_to_key).collect());
    }

    Tasks { paths, map, deps }
}
