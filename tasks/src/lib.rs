use ninja_desc::ast::*;
use std::{collections::HashMap, fmt::Display};

#[derive(Debug, PartialOrd, Ord, Hash, Eq, PartialEq, Clone)]
pub enum Key {
    Single(Vec<u8>),
    Multi(Vec<Key>),
}

impl Key {
    pub fn is_single(&self) -> bool {
        matches!(self, Key::Single(_))
    }

    pub fn is_multi(&self) -> bool {
        matches!(self, Key::Multi(_))
    }

    pub fn as_bytes(&self) -> &[u8] {
        match *self {
            Key::Single(ref bytes) => bytes,
            _ => panic!("only works on Key::Single"),
        }
    }
}

impl Display for Key {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Key::Single(v) => write!(
                f,
                "Key::Single({})",
                std::str::from_utf8(v).map_err(|_| std::fmt::Error {})?
            ),
            Key::Multi(vs) => {
                write!(f, "Key::Multi(")?;
                for v in vs {
                    write!(f, "{},", v)?;
                }
                write!(f, ")")
            }
        }
    }
}

#[derive(Debug)]
pub enum TaskVariant {
    Source,
    // Indicates that this key just depends on another, usually Multi key.
    // Also used to map Phony.
    Retrieve,
    Command(String),
}

pub type Dependencies = Vec<Key>;

#[derive(Debug)]
pub struct Task {
    pub dependencies: Dependencies,
    pub variant: TaskVariant,
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
            TaskVariant::Command(ref s) => Some(s),
            _ => None,
        }
    }
}

pub type TasksMap = HashMap<Key, Task>;

#[derive(Debug)]
pub struct Tasks {
    map: TasksMap,
}

impl Tasks {
    pub fn task(&self, key: &Key) -> Option<&Task> {
        self.map.get(key)
    }

    pub fn all_tasks(&self) -> &TasksMap {
        &self.map
    }
}

impl Display for Tasks {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let format_single = |key: &Key| -> String {
            if let Key::Single(ref bytes) = key {
                format!("{}", std::str::from_utf8(bytes).unwrap())
            } else {
                panic!("Should not come here");
            }
        };

        let write_key = |f: &mut std::fmt::Formatter<'_>, key: &Key| -> std::fmt::Result {
            match key {
                Key::Single(_) => write!(f, "{}", format_single(key)),
                Key::Multi(ref syms) => write!(
                    f,
                    "Multi{:?}",
                    syms.iter().map(format_single).collect::<Vec<String>>()
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
            write!(f, "]\n")?;
        }
        write!(f, "}}")
    }
}

fn sym_to_key(output: Vec<u8>) -> Key {
    Key::Single(output)
}

fn syms_to_key(mut outputs: Vec<Vec<u8>>) -> Key {
    assert!(outputs.len() > 1);
    // TODO: This isn't perfect because we want to show any errors to the user in the order in
    // which they originally wrote the build rule, and not what we determine to be the order.
    outputs.sort();
    Key::Multi(outputs.iter().map(|o| sym_to_key(o.clone())).collect())
}

pub fn description_to_tasks(desc: Description) -> Tasks {
    let mut map: TasksMap = HashMap::new();
    // Since no two build edges can produce any single output, they also cannot produce any
    // multi-outputs. This means every build's outputs are guaranteed to be unique and we may as
    // well create a new key for each.
    for build in desc.builds {
        let key = if build.outputs.len() == 1 {
            sym_to_key((&build.outputs[0]).clone())
        } else {
            let main_key = syms_to_key(build.outputs);
            if let Key::Multi(ref keys) = main_key {
                for key in keys {
                    map.insert(
                        key.clone(),
                        Task {
                            dependencies: vec![main_key.clone()],
                            variant: TaskVariant::Retrieve,
                        },
                    );
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

    Tasks { map }
}
