extern crate ninja_desc;
extern crate ninja_interface;
extern crate ninja_paths;
extern crate petgraph;

use ninja_paths::PathCache;

use std::{collections::HashSet, ffi::OsStr, fs::metadata, os::unix::ffi::OsStrExt};

use petgraph::{
    graph::NodeIndex,
    visit::{depth_first_search, Control, DfsEvent, DfsPostOrder},
    Direction,
};

use ninja_desc::{BuildGraph, Key, TaskResult, TasksMap};
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
        // Cannot use depth_first_search which doesn't say if it is postorder.
        let mut visitor = DfsPostOrder::empty(self.graph);
        for start in start {
            visitor.move_to(start);
            while let Some(node) = visitor.next(self.graph) {
                order.push(node);
            }
        }
        // TODO: How to model parallel task execution?
        // At this point we know exactly which tasks to run, and in order of their dependencies,
        // but as soon as we start running in parallel, we've to wait for the task for foo.o to
        // finish before starting the task for foo, without relying on the serialization right
        // here. So instead of doing a straight topo sort above, are we going back to a ninja style
        // ready list? Alternatively we simply build up a Future tree while we are topologically
        // sorting, then queue them up using a thread pool.
        // As a thought experiment, if we did this from scratch, how would it work? We cannot
        // submit work to the threadpool until it can actually be done. similarly, we need to hear
        // back from the thread pool when work finishes so we can check if any tasks can be kicked
        // off.
        for node in order {
            let task = self.tasks.get(&node);
            if let Some(task) = task {
                rebuilder.build(node, TaskResult {}, task.as_ref());
            }
        }
    }
}

#[derive(Debug)]
pub struct MTimeRebuilder<'a> {
    graph: &'a BuildGraph,
    path_cache: PathCache,
}

impl<'a> MTimeRebuilder<'a> {
    pub fn new(graph: &BuildGraph, path_cache: PathCache) -> MTimeRebuilder {
        MTimeRebuilder { graph, path_cache }
    }
}

impl<'a> Rebuilder<NodeIndex, TaskResult> for MTimeRebuilder<'a> {
    fn build(
        &self,
        node: NodeIndex,
        _current_value: TaskResult,
        task: &dyn Task<TaskResult>,
    ) -> TaskResult {
        // This function obviously needs a lot of error handling.
        let key = &self.graph[node];
        let mtime = match key {
            Key::Path(path_ref) => {
                let path_str: &OsStr = OsStrExt::from_bytes(self.path_cache.get(*path_ref));
                let path = std::path::Path::new(path_str);
                if path.exists() {
                    Some(metadata(path).expect("metadata").modified().expect("mtime"))
                } else {
                    None
                }
            }
            Key::Multi(outputs) => {
                // If the oldest output is older than any input, rebuild.
                let times: Vec<std::time::SystemTime> = outputs
                    .iter()
                    .filter_map(|path_ref| {
                        let path_str: &OsStr = OsStrExt::from_bytes(self.path_cache.get(*path_ref));
                        let path = std::path::Path::new(path_str);
                        if path.exists() {
                            Some(metadata(path).expect("metadata").modified().expect("mtime"))
                        } else {
                            None
                        }
                    })
                    .collect();
                if times.len() < outputs.len() {
                    // At least one output did not exist, so always build.
                    None
                } else {
                    Some(times.into_iter().min().expect("at least one"))
                }
            }
        };
        let dirty = if mtime.is_none() {
            true
        } else {
            let mtime = mtime.unwrap();
            let mut dependencies = self.graph.neighbors_directed(node, Direction::Outgoing);
            dependencies.any(|dep| {
                let dep = &self.graph[dep];
                match dep {
                    Key::Path(path_ref) => {
                        let path_str: &OsStr = OsStrExt::from_bytes(self.path_cache.get(*path_ref));
                        let dep_mtime = metadata(path_str)
                            .expect("metadata")
                            .modified()
                            .expect("mtime");
                        dep_mtime > mtime
                    }
                    Key::Multi(_) => {
                        // TODO: assert task is phony.
                        true
                    }
                }
            })
        };
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

#[derive(Debug)]
pub struct BuildLog {}

impl BuildLog {
    pub fn read() -> BuildState {
        BuildState {}
    }
}
