extern crate ninja_desc;
extern crate ninja_interface;
extern crate petgraph;

use std::collections::HashSet;

use petgraph::{
    graph::NodeIndex,
    visit::{depth_first_search, Control, DfsEvent},
};

use ninja_desc::{BuildGraph, TaskResult, TasksMap};
use ninja_interface::{Rebuilder, Scheduler, Task};

#[derive(Debug)]
pub struct BuildState {}

#[derive(Debug)]
pub struct TopoScheduler<'a> {
    graph: &'a BuildGraph,
    tasks: TasksMap,
}

impl<'a> TopoScheduler<'a> {
    pub fn new(graph: &'a BuildGraph, tasks: TasksMap) -> TopoScheduler {
        TopoScheduler { graph, tasks }
    }
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
            let key = &self.graph[node];
            let task = self.tasks.get(key);
            if let Some(task) = task {
                rebuilder.build(node, TaskResult {}, task.as_ref());
            }
        }
    }
}

#[derive(Debug)]
pub struct MTimeRebuilder<'a> {
    graph: &'a BuildGraph,
}

impl<'a> MTimeRebuilder<'a> {
    pub fn new(graph: &BuildGraph) -> MTimeRebuilder {
        MTimeRebuilder { graph }
    }
}

impl<'a> Rebuilder<NodeIndex, TaskResult> for MTimeRebuilder<'a> {
    fn build(
        &self,
        key: NodeIndex,
        _current_value: TaskResult,
        task: &dyn Task<TaskResult>,
    ) -> TaskResult {
        todo!("IMPL");
        /*
            let mut desc = self.desc.borrow_mut();
            let dirty = desc.dirty(target);

            if dirty {
                // Run command.
                let command = desc.command(target);
                if command.is_none() {
                    return;
                }
                let command_str: &OsStr = OsStrExt::from_bytes(command.unwrap());
                println!("{}", std::str::from_utf8(command.unwrap()).unwrap());
                // POSIX only
                Command::new("/bin/sh")
                    .arg("-c")
                    .arg(command_str)
                    .status()
                    .expect("success");
            }

            desc.mark_done(target);
        */
    }
}

#[derive(Debug)]
pub struct BuildLog {}

impl BuildLog {
    pub fn read() -> BuildState {
        BuildState {}
    }
}
