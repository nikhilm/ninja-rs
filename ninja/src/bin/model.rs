use std::{collections::HashSet, fs::metadata, process::Command};

extern crate petgraph;
use petgraph::{
    graph::NodeIndex,
    visit::{depth_first_search, Control, DfsEvent},
    Direction, Graph,
};

trait Task<V> {
    fn run(&self /*, fetch: Fetch<K, V>*/) -> V;
}

trait Rebuilder<K, V> {
    fn build(&self, key: K, current_value: V, task: &dyn Task<V>) -> V;
}

trait Scheduler<K, V> {
    fn schedule(&self, rebuilder: &dyn Rebuilder<K, V>, start: Vec<K>);
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
}

impl CommandTask {
    fn new<S: Into<String>>(c: S) -> CommandTask {
        CommandTask { command: c.into() }
    }
}

impl Task<TaskResult> for CommandTask {
    fn run(&self) -> TaskResult {
        eprintln!("{}", &self.command);
        Command::new("/bin/sh")
            .arg("-c")
            .arg(&self.command)
            .status()
            .expect("success");
        TaskResult {}
    }
}

#[derive(Debug)]
struct PhonyTask {}

impl PhonyTask {
    fn new() -> PhonyTask {
        PhonyTask {}
    }
}

impl Task<TaskResult> for PhonyTask {
    fn run(&self) -> TaskResult {
        TaskResult {}
    }
}

type MyGraph = petgraph::Graph<Key, ()>;
struct MTimeRebuilder<'a> {
    graph: &'a MyGraph,
}

impl<'a> Rebuilder<NodeIndex, TaskResult> for MTimeRebuilder<'a> {
    fn build(&self, node: NodeIndex, _: TaskResult, task: &dyn Task<TaskResult>) -> TaskResult {
        let key = &self.graph[node];
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
        let mut dependencies = self.graph.neighbors_directed(node, Direction::Outgoing);
        let dirty = dependencies.any(|dep| {
            let dep = &self.graph[dep];
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

struct TopoScheduler<'a> {
    tasks: &'a dyn Fn(NodeIndex) -> Option<&'a dyn Task<TaskResult>>,
    graph: &'a MyGraph,
}

impl<'a> Scheduler<NodeIndex, TaskResult> for TopoScheduler<'a> {
    fn schedule(&self, rebuilder: &dyn Rebuilder<NodeIndex, TaskResult>, start: Vec<NodeIndex>) {
        let mut order: Vec<NodeIndex> = Vec::new();
        // might be able to use CrossForwardEdge instead of this to detect cycles.
        let mut seen: HashSet<NodeIndex> = HashSet::new();
        let cycle_checking_sorter = |evt: DfsEvent<NodeIndex>| -> Control<()> {
            if let DfsEvent::Finish(n, _) = evt {
                if seen.contains(&n) {
                    eprintln!("Seen {:?} already", &self.graph[n]);
                    panic!("cycle");
                }
                seen.insert(n);
                order.push(n);
            }
            Control::Continue
        };
        depth_first_search(self.graph, start.into_iter(), cycle_checking_sorter);
        for node in order {
            let task = (self.tasks)(node);
            if let Some(task) = task {
                rebuilder.build(node, TaskResult {}, task);
            }
        }
    }
}

fn main() {
    // Graph is flipped to accomodate dfs topo sort
    let mut graph: Graph<Key, ()> = petgraph::Graph::new();

    let source_key = Key::Path("foo.c".into());
    let source_node = graph.add_node(source_key);

    let cc_task = CommandTask::new("gcc -c foo.c");
    let cc_key = Key::Path("foo.o".into());
    let cc_node = graph.add_node(cc_key);
    graph.add_edge(cc_node, source_node, ());

    let link_task = CommandTask::new("gcc -o foo foo.o");
    let link_key = Key::Path("foo".into());
    let link_node = graph.add_node(link_key);
    graph.add_edge(link_node, cc_node, ());

    let a_key = Key::Path("a".into());
    let a_node = graph.add_node(a_key);
    let touch_task = CommandTask::new("touch b c");
    let phony_b = PhonyTask::new();
    let phony_c = PhonyTask::new();
    let multi_key = Key::Multi(vec!["b".into(), "c".into()]);
    let multi_node = graph.add_node(multi_key);
    let phony_b_key = Key::Path("b".into());
    let phony_b_node = graph.add_node(phony_b_key);
    let phony_c_key = Key::Path("c".into());
    let phony_c_node = graph.add_node(phony_c_key);
    graph.add_edge(multi_node, a_node, ());
    graph.add_edge(phony_b_node, multi_node, ());
    graph.add_edge(phony_c_node, multi_node, ());

    let tasks = |k: NodeIndex| -> Option<&dyn Task<TaskResult>> {
        if k == cc_node {
            Some(&cc_task)
        } else if k == link_node {
            Some(&link_task)
        } else if k == phony_b_node {
            Some(&phony_b)
        } else if k == phony_c_node {
            Some(&phony_c)
        } else if k == multi_node {
            Some(&touch_task)
        } else {
            None
        }
    };

    // Fetch is unused because it really doesn't make any sense and introduces too many closures
    // kind of things where rebuilder now has to return a pretend-task that calls the underlying
    // task.

    let rebuilder = MTimeRebuilder { graph: &graph };
    let scheduler = TopoScheduler {
        tasks: &tasks,
        graph: &graph,
    };
    let start: Vec<NodeIndex> = graph.externals(Direction::Incoming).collect();
    scheduler.schedule(&rebuilder, start);
}
