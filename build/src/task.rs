/*
 * Copyright 2020 Nikhil Marathe <nsm.nikhil@gmail.com>
 *
 * Licensed under the Apache License, Version 2.0 (the "License");
 * you may not use this file except in compliance with the License.
 * You may obtain a copy of the License at
 *
 *     http://www.apache.org/licenses/LICENSE-2.0
 *
 * Unless required by applicable law or agreed to in writing, software
 * distributed under the License is distributed on an "AS IS" BASIS,
 * WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
 * See the License for the specific language governing permissions and
 * limitations under the License.
 */

use std::{collections::HashMap, fmt::Display, ops::Deref};

use ninja_parse::repr::*;

#[derive(Debug, PartialOrd, Ord, Hash, Eq, PartialEq, Clone)]
pub struct KeyPath(Vec<u8>);

impl From<Vec<u8>> for KeyPath {
    fn from(v: Vec<u8>) -> Self {
        KeyPath(v)
    }
}

impl KeyPath {
    pub fn as_bytes(&self) -> &[u8] {
        self.0.as_slice()
    }
}

impl Display for KeyPath {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Path({})",
            std::str::from_utf8(&self.0).map_err(|_| std::fmt::Error {})?
        )
    }
}

#[derive(Debug, PartialOrd, Ord, Hash, Eq, PartialEq, Clone)]
pub struct KeyMulti(Vec<KeyPath>);

impl From<Vec<KeyPath>> for KeyMulti {
    fn from(v: Vec<KeyPath>) -> Self {
        KeyMulti(v)
    }
}

impl Display for KeyMulti {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Multi(")?;
        for v in &self.0 {
            write!(f, "{},", v)?;
        }
        write!(f, ")")
    }
}

impl Deref for KeyMulti {
    type Target = [KeyPath];

    fn deref(&self) -> &Self::Target {
        self.0.deref()
    }
}

#[derive(Debug, PartialOrd, Ord, Hash, Eq, PartialEq, Clone)]
pub enum Key {
    Path(KeyPath),
    Multi(KeyMulti),
}

impl Key {
    pub fn is_path(&self) -> bool {
        matches!(self, Key::Path(_))
    }

    pub fn is_multi(&self) -> bool {
        matches!(self, Key::Multi(_))
    }

    pub fn iter(&self) -> impl Iterator<Item = &KeyPath> {
        match self {
            Key::Path(p) => std::iter::once(p),
            Key::Multi(_) => panic!(),
            //Key::Multi(vs) => { Box::new( vs.iter().map(|v| v.iter()).flatten() )},
        }
    }
}

impl Display for Key {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Key::Path(p) => write!(f, "Key({})", p),
            Key::Multi(ks) => write!(f, "Key({})", ks),
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
    pub order_dependencies: Dependencies,
    pub variant: TaskVariant,
}

impl Task {
    pub fn dependencies(&self) -> &[Key] {
        &self.dependencies
    }

    pub fn order_dependencies(&self) -> &[Key] {
        &self.order_dependencies
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
        write!(f, "Tasks{{\n tasks:\n")?;
        let mut keys: Vec<&Key> = self.map.keys().collect();
        keys.sort();
        for key in keys {
            let task = &self.map[key];
            write!(f, "  {}", key)?;
            write!(f, " -> {:?} [", task.variant)?;
            for key in task.dependencies() {
                write!(f, "{}, ", key)?;
            }
            writeln!(f, "]")?;
        }
        write!(f, "}}")
    }
}

fn path_to_key(path: Vec<u8>) -> KeyPath {
    KeyPath(path)
}

fn paths_to_multi_key(mut outputs: Vec<Vec<u8>>) -> KeyMulti {
    assert!(outputs.len() > 1);
    // TODO: This isn't perfect because we want to show any errors to the user in the order in
    // which they originally wrote the build rule, and not what we determine to be the order.
    outputs.sort();
    KeyMulti(outputs.iter().map(|o| path_to_key(o.clone())).collect())
}

pub fn description_to_tasks_with_start(
    desc: Description,
    start: Option<Vec<Vec<u8>>>,
) -> (Tasks, Option<Vec<KeyPath>>) {
    let requested = if let Some(specified) = start {
        Some(specified.into_iter().map(path_to_key).collect())
    } else {
        desc.defaults
            .map(|v| v.into_iter().map(path_to_key).collect())
    };
    let mut map: TasksMap = HashMap::new();
    // Since no two build edges can produce any single output, they also cannot produce any
    // multi-outputs. This means every build's outputs are guaranteed to be unique and we may as
    // well create a new key for each.
    for build in desc.builds {
        let key = if build.outputs.len() == 1 {
            Key::Path(path_to_key((&build.outputs[0]).clone()))
        } else {
            let main_key = paths_to_multi_key(build.outputs);
            for key in main_key.deref() {
                map.insert(
                    Key::Path(key.clone()),
                    Task {
                        dependencies: vec![Key::Multi(main_key.clone())],
                        order_dependencies: vec![],
                        variant: TaskVariant::Retrieve,
                    },
                );
            }
            Key::Multi(main_key)
        };
        map.insert(
            key.clone(),
            Task {
                dependencies: build
                    .inputs
                    .into_iter()
                    .map(path_to_key)
                    .map(Key::Path)
                    .chain(
                        build
                            .implicit_inputs
                            .into_iter()
                            .map(path_to_key)
                            .map(Key::Path),
                    )
                    .collect(),
                order_dependencies: build
                    .order_inputs
                    .into_iter()
                    .map(path_to_key)
                    .map(Key::Path)
                    .collect(),
                variant: match build.action {
                    Action::Phony => TaskVariant::Retrieve,
                    Action::Command(s) => TaskVariant::Command(s),
                },
            },
        );
    }

    (Tasks { map }, requested)
}

pub fn description_to_tasks(desc: Description) -> (Tasks, Option<Vec<KeyPath>>) {
    description_to_tasks_with_start(desc, None)
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    #[should_panic]
    fn test_paths_to_multi_key_1() {
        paths_to_multi_key(vec![b"a".to_vec()]);
    }

    #[test]
    fn test_paths_to_multi_key_at_least_2() {
        paths_to_multi_key(vec![b"a".to_vec(), b"b".to_vec()]);
    }

    #[test]
    fn test_sort() {
        let key = paths_to_multi_key(vec![
            b"hello".to_vec(),
            b"grungy".to_vec(),
            b"aaaaaaaaaaaaaaaa.txt".to_vec(),
        ]);

        let mut iter = key.iter().peekable();
        while let Some(elem) = iter.next() {
            if let Some(next) = iter.peek() {
                assert!(elem <= next);
            }
        }
    }

    #[test]
    fn test_outputs_processing() {
        let desc = Description {
            builds: vec![Build {
                action: Action::Command("compiler".to_owned()),
                inputs: vec![],
                implicit_inputs: vec![],
                order_inputs: vec![],
                outputs: vec![b"output9.txt".to_vec(), b"output2.txt".to_vec()],
            }],
            defaults: None,
        };

        let (tasks, _) = description_to_tasks(desc);
        assert_eq!(tasks.all_tasks().len(), 3);

        // find the multi.
        let mut found_multi = false;
        let mut single_count = 0;
        for key in tasks.all_tasks().keys() {
            if let Key::Multi(keys) = key {
                found_multi = true;
                assert_eq!(
                    keys.0,
                    vec![
                        KeyPath(b"output2.txt".to_vec()),
                        KeyPath(b"output9.txt".to_vec())
                    ]
                );
                let task = tasks.task(key).expect("valid task");
                assert!(task.is_command());
                assert!(task.dependencies().is_empty());
            } else if let Key::Path(path) = key {
                single_count += 1;
                assert!((path.as_bytes() == b"output2.txt" || path.as_bytes() == b"output9.txt"));

                let task = tasks.task(key).expect("valid task");
                assert!(task.is_retrieve());
                assert_eq!(task.dependencies().len(), 1);
                let dep = task.dependencies()[0].clone();
                assert!(matches!(dep, Key::Multi(_)));
            }
        }
        assert!(found_multi);
        assert_eq!(single_count, 2);
    }

    #[test]
    fn implicit_dependencies() {
        let desc = Description {
            builds: vec![Build {
                action: Action::Command("compiler".to_owned()),
                inputs: vec![b"a.txt".to_vec(), b"b.txt".to_vec()],
                implicit_inputs: vec![b"c.txt".to_vec(), b"d.txt".to_vec()],
                order_inputs: vec![],
                outputs: vec![b"z.txt".to_vec()],
            }],
            defaults: None,
        };

        let (tasks, _) = description_to_tasks(desc);
        assert_eq!(tasks.all_tasks().len(), 1);
        let task = tasks
            .task(&Key::Path(KeyPath(b"z.txt".to_vec())))
            .expect("valid task");
        assert!(task.is_command());
        assert_eq!(task.dependencies().len(), 4);
    }

    #[test]
    fn order_dependencies() {
        let desc = Description {
            builds: vec![Build {
                action: Action::Command("compiler".to_owned()),
                inputs: vec![b"a.txt".to_vec(), b"b.txt".to_vec()],
                implicit_inputs: vec![],
                order_inputs: vec![b"c.txt".to_vec(), b"d.txt".to_vec()],
                outputs: vec![b"z.txt".to_vec()],
            }],
            defaults: None,
        };

        let (tasks, _) = description_to_tasks(desc);
        assert_eq!(tasks.all_tasks().len(), 1);
        let task = tasks
            .task(&Key::Path(KeyPath(b"z.txt".to_vec())))
            .expect("valid task");
        assert!(task.is_command());
        assert_eq!(task.dependencies().len(), 2);
        assert_eq!(task.order_dependencies().len(), 2);
    }
}
