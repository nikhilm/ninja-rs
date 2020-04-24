use std::{fs::metadata, process::Command};

type Fetch<K, V> = Box<dyn Fn(K) -> V>;

struct TaskResult<R>(R);

trait Task<K, V, R> {
    fn run(&self /*, fetch: Fetch<K, V>*/) -> TaskResult<R>;
    fn dependencies(&self) -> Vec<K>;
}

trait Rebuilder<K, V, R> {
    fn build(&self, key: K, currentValue: V, task: impl Task<K, V, R>) -> TaskResult<R>;
}

#[derive(Debug)]
struct CommandTask {
    command: String,
    pub dependencies: Vec<String>,
}

type CommandFetch = Fetch<String, ()>;
// actually needs a buffer result or something.
type CommandTaskResult = TaskResult<()>;

impl CommandTask {
    fn new<S: Into<String>>(c: S) -> CommandTask {
        CommandTask {
            command: c.into(),
            dependencies: Vec::new(),
        }
    }

    fn add_dep<S: Into<String>>(&mut self, c: S) {
        self.dependencies.push(c.into());
    }
}

impl Task<String, (), ()> for &CommandTask {
    fn run(&self) -> TaskResult<()> {
        eprintln!("{}", &self.command);
        Command::new("/bin/sh")
            .arg("-c")
            .arg(&self.command)
            .status()
            .expect("success");
        TaskResult(())
    }

    fn dependencies(&self) -> Vec<String> {
        self.dependencies.clone()
    }
}

struct MTimeRebuilder {}

impl Rebuilder<String, (), ()> for MTimeRebuilder {
    fn build(&self, key: String, _: (), task: impl Task<String, (), ()>) -> TaskResult<()> {
        let mtime = metadata(key).expect("metadata").modified().expect("mtime");
        let dirty = task.dependencies().iter().any(|dep| {
            let dep_mtime = metadata(dep).expect("metadata").modified().expect("mtime");
            dep_mtime > mtime
        });
        if dirty {
            // TODO: actually need some return type that can failure to run this task if the
            // dependency is not available.
            // may want different response based on dep being source vs intermediate. for
            // intermediate, whatever should've produced it will fail and have the error message.
            // So fail with not found if not a known output.
            task.run();
        }
        TaskResult(())
    }
}

fn main() {
    let mut cc_task = CommandTask::new("gcc -c foo.c");
    cc_task.add_dep("foo.c");
    let mut link_task = CommandTask::new("gcc -o foo foo.o");
    link_task.add_dep("foo.o");
    let tasks = |k: &str| {
        if k == "foo.o" {
            Some(&cc_task)
        } else if k == "foo" {
            Some(&link_task)
        } else {
            None
        }
    };

    // pretend scheduler got this order.
    let order = vec!["foo.o", "foo"];

    // Fetch is unused because it really doesn't make any sense and introduces too many closures
    // kind of things where rebuilder now has to return a pretend-task that calls the underlying
    // task.

    let rebuilder = MTimeRebuilder {};
    for k in order {
        rebuilder.build(k.clone().to_string(), (), tasks(k).unwrap());
    }
}
