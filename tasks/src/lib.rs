use ninja_desc::ast::*;
use std::{collections::HashMap, fmt::Display};

#[derive(Debug, PartialOrd, Ord, Hash, Eq, PartialEq, Clone)]
pub enum Key {
    Single(Sym),
    Multi(Vec<Sym>),
}

#[derive(Debug)]
pub enum TaskVariant {
    // Indicates that this key just depends on another, usually Multi key.
    // Also used to map Phony.
    Retrieve,
    Command(String),
}

pub type Dependencies = Vec<Key>;

#[derive(Debug)]
pub struct Task {
    dependencies: Dependencies,
    variant: TaskVariant,
}

impl Task {
    pub fn dependencies(&self) -> &[Key] {
        &self.dependencies
    }

    pub fn is_retrieve(&self) -> bool {
        std::matches!(self.variant, TaskVariant::Retrieve)
    }

    pub fn is_command(&self) -> bool {
        std::matches!(self.variant, TaskVariant::Command(_))
    }

    pub fn command(&self) -> Option<&String> {
        match self.variant {
            TaskVariant::Retrieve => None,
            TaskVariant::Command(ref s) => Some(s),
        }
    }
}

pub type TasksMap = HashMap<Key, Task>;

#[derive(Debug)]
pub struct Tasks {
    paths: DefaultStringInterner,
    map: TasksMap,
}

impl Tasks {
    pub fn path_for(&self, key: &Key) -> Option<&str> {
        match key {
            Key::Single(s) => self.paths.resolve(*s),
            Key::Multi(_) => None,
        }
    }

    pub fn task(&self, key: &Key) -> Option<&Task> {
        self.map.get(key)
    }

    pub fn all_tasks(&self) -> &TasksMap {
        &self.map
    }
}

impl Display for Tasks {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let write_key = |f: &mut std::fmt::Formatter<'_>, key: &Key| -> std::fmt::Result {
            match key {
                Key::Single(sym) => write!(f, "{}", self.paths.resolve(*sym).unwrap()),
                Key::Multi(syms) => write!(
                    f,
                    "Multi{:?}",
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
            let task = &self.map[key];
            write!(f, "  ")?;
            write_key(f, key)?;
            write!(f, " -> {:?} [", task.variant)?;
            for key in task.dependencies() {
                write_key(f, key)?;
                write!(f, ", ")?;
            }
            write!(f, "]\n");
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
        let key = if build.outputs.len() == 1 {
            sym_to_key(build.outputs[0])
        } else {
            let main_key = syms_to_key(build.outputs);
            if let Key::Multi(ref syms) = main_key {
                for sym in syms {
                    let key = sym_to_key(*sym);
                    map.insert(
                        key.clone(),
                        Task {
                            dependencies: vec![main_key.clone()],
                            variant: TaskVariant::Retrieve,
                        },
                    );
                    // TODO: Stop cloning.
                    deps.insert(key, vec![main_key.clone()]);
                }
            } else {
                unreachable!();
            }
            main_key
        };
        map.insert(
            key.clone(),
            Task {
                dependencies: build.inputs.into_iter().map(sym_to_key).collect(),
                variant: match build.action {
                    Action::Phony => TaskVariant::Retrieve,
                    Action::Command(s) => TaskVariant::Command(s),
                },
            },
        );
    }

    Tasks { paths, map }
}
