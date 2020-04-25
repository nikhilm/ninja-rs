use std::{fs::metadata, process::Command};

type Fetch<K, V> = Box<dyn Fn(K) -> V>;

trait Task<K, V> {
    fn run(&self /*, fetch: Fetch<K, V>*/) -> V;
    fn dependencies(&self) -> Vec<K>;
}

trait Rebuilder<K, V> {
    fn build(&self, key: K, currentValue: V, task: &dyn Task<K, V>) -> V;
}

// --------- impl

#[derive(Debug, PartialEq, Eq, Hash, Clone)]
enum Key {
    Path(String),
    Multi(Vec<String>),
}

impl From<String> for Key {
    fn from(s: String) -> Key {
        Key::Path(s)
    }
}

impl From<&str> for Key {
    fn from(s: &str) -> Key {
        Key::Path(s.to_string())
    }
}

// actually needs a buffer result or something.
struct TaskResult {}

#[derive(Debug)]
struct CommandTask {
    command: String,
    pub dependencies: Vec<Key>,
}

impl CommandTask {
    fn new<S: Into<String>>(c: S) -> CommandTask {
        CommandTask {
            command: c.into(),
            dependencies: Vec::new(),
        }
    }

    fn add_dep<S: Into<Key>>(&mut self, c: S) {
        self.dependencies.push(c.into());
    }
}

impl Task<Key, TaskResult> for CommandTask {
    fn run(&self) -> TaskResult {
        eprintln!("{}", &self.command);
        Command::new("/bin/sh")
            .arg("-c")
            .arg(&self.command)
            .status()
            .expect("success");
        TaskResult {}
    }

    fn dependencies(&self) -> Vec<Key> {
        self.dependencies.clone()
    }
}

#[derive(Debug)]
struct PhonyTask {
    pub dependencies: Vec<Key>,
}

impl PhonyTask {
    fn new() -> PhonyTask {
        PhonyTask {
            dependencies: Vec::new(),
        }
    }

    fn add_dep<S: Into<Key>>(&mut self, c: S) {
        self.dependencies.push(c.into());
    }
}

impl Task<Key, TaskResult> for PhonyTask {
    fn run(&self) -> TaskResult {
        TaskResult {}
    }

    fn dependencies(&self) -> Vec<Key> {
        self.dependencies.clone()
    }
}

struct MTimeRebuilder {}

impl Rebuilder<Key, TaskResult> for MTimeRebuilder {
    fn build(&self, key: Key, _: TaskResult, task: &dyn Task<Key, TaskResult>) -> TaskResult {
        let outputs: Vec<String> = match key {
            Key::Path(p) => vec![p.clone()],
            Key::Multi(ps) => ps.clone(),
        };
        // If the oldest output is older than any input, rebuild.
        let mtime = outputs
            .iter()
            .map(|path| metadata(path).expect("metadata").modified().expect("mtime"))
            .min()
            .expect("at least one");
        let dirty = task.dependencies().iter().any(|dep| {
            match dep {
                Key::Path(p) => {
                    let dep_mtime = metadata(p).expect("metadata").modified().expect("mtime");
                    dep_mtime > mtime
                }
                Key::Multi(_) => {
                    // TODO: assert task is phony.
                    true
                }
            }
        });
        if dirty {
            // TODO: actually need some return type that can failure to run this task if the
            // dependency is not available.
            // may want different response based on dep being source vs intermediate. for
            // intermediate, whatever should've produced it will fail and have the error message.
            // So fail with not found if not a known output.
            task.run();
        }
        TaskResult {}
    }
}

fn main() {
    let multi_key = Key::Multi(vec!["b".into(), "c".into()]);
    let mut cc_task = CommandTask::new("gcc -c foo.c");
    cc_task.add_dep("foo.c");
    let mut link_task = CommandTask::new("gcc -o foo foo.o");
    link_task.add_dep("foo.o");
    let mut touch_task = CommandTask::new("touch b c");
    touch_task.add_dep("a");
    let mut phony_b = PhonyTask::new();
    phony_b.add_dep(multi_key.clone());
    let mut phony_c = PhonyTask::new();
    phony_c.add_dep(multi_key.clone());
    let tasks = |k: Key| -> Option<&dyn Task<Key, TaskResult>> {
        if k == "foo.o".into() {
            Some(&cc_task)
        } else if k == "foo".into() {
            Some(&link_task)
        } else if k == "b".into() {
            Some(&phony_b)
        } else if k == "c".into() {
            Some(&phony_c)
        } else if k == multi_key {
            Some(&touch_task)
        } else {
            None
        }
    };

    // pretend scheduler got this order.
    let order: Vec<Key> = vec![
        Key::Multi(vec!["b".into(), "c".into()]),
        "b".into(),
        "foo.o".into(),
        "foo".into(),
    ];

    // Fetch is unused because it really doesn't make any sense and introduces too many closures
    // kind of things where rebuilder now has to return a pretend-task that calls the underlying
    // task.

    let rebuilder = MTimeRebuilder {};
    for k in order {
        rebuilder.build(k.clone(), TaskResult {}, tasks(k).unwrap());
    }
}
