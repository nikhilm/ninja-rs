extern crate ninja_desc;
extern crate ninja_interface;
extern crate ninja_paths;
extern crate petgraph;

use std::collections::{HashSet, VecDeque};

use petgraph::{graph::NodeIndex, visit::DfsPostOrder, Direction};

use ninja_desc::{BuildGraph, TaskResult, TasksMap};
use ninja_interface::{Rebuilder, Scheduler};

mod rebuilder;
pub use rebuilder::MTimeRebuilder;

#[derive(Debug)]
struct Pool {
    capacity: u32,
    in_use: u32,
}

impl Default for Pool {
    fn default() -> Self {
        // yea this is obviously dumb.
        Self::with_capacity(8)
    }
}

// acquiring and giving up can probably leverage the type system to have a MutexGuard kind of
// thing.
impl Pool {
    pub fn with_capacity(capacity: u32) -> Pool {
        Pool {
            capacity,
            in_use: 0,
        }
    }

    pub fn has_capacity(&mut self) -> bool {
        return self.in_use < self.capacity;
    }

    pub fn acquire(&mut self) {
        assert!(self.has_capacity());
        self.in_use += 1;
    }

    pub fn release(&mut self) {
        assert!(self.in_use > 0);
        self.in_use -= 1;
    }
}

#[derive(Debug, Default)]
struct BuildState {
    pub ready: VecDeque<NodeIndex>,
    pub waiting: HashSet<NodeIndex>,
    pub building: HashSet<NodeIndex>,

    pub global_pool: Pool,
}

#[derive(Debug)]
pub struct ParallelTopoScheduler<'a> {
    graph: &'a BuildGraph,
    tasks: TasksMap,
}

impl<'a> ParallelTopoScheduler<'a> {
    pub fn new(graph: &'a BuildGraph, tasks: TasksMap) -> Self {
        Self { graph, tasks }
    }
}

impl<'a> Scheduler<NodeIndex, TaskResult> for ParallelTopoScheduler<'a> {
    fn schedule(&self, _rebuilder: &dyn Rebuilder<NodeIndex, TaskResult>, start: Vec<NodeIndex>) {
        let mut build_state: BuildState = Default::default();

        // Cannot use depth_first_search which doesn't say if it is postorder.
        let mut visitor = DfsPostOrder::empty(self.graph);
        for start in start {
            visitor.move_to(start);
            while let Some(node) = visitor.next(self.graph) {
                if self.graph.edges_directed(node, Direction::Outgoing).count() == 0 {
                    build_state.ready.push_back(node);
                } else {
                    build_state.waiting.insert(node);
                }
            }
        }

        loop {
            if build_state.global_pool.has_capacity() {
                if let Some(next) = build_state.ready.pop_front() {
                    build_state.global_pool.acquire();
                    // Rebuilder interface also needs to change to be more "async". Like build only
                    // starts a task.
                    rebuilder.build
                }
            }
        }

        /*let has_capacity = || -> bool {
            true // TODO: pools etc.
        };

        let schedule_work = || {
            let node = ready.pop_front();
            let task = self.tasks.get(&node);
            if let Some(task) = task {
                build_state.building.insert(node);
                rebuilder.build(node, TaskResult {}, task.as_ref());
            } else {
                // input. we may want to check it actually exists on disk?
                // OR that can be a task with a task result that we should insert.
            }
        };

        let check_maybe_ready = |node| ->bool {
            // some kind of counter associated with this.
            // All of this is leading to some mutable state that the rebuilder should store, either
            // in this function's scope (preferred) or as a member.
            false
        };

        let mark_done = |node| {
            assert!(building.contains(node));
            // TODO: analyze the result for errors etc.
            let dependents = self.graph.neighbors_directed(node, Direction::Incoming);
            for dependent in dependents {
                assert!(waiting.contains(dependent));
                if check_maybe_ready(dependent) {
                    ready.push_back(dependent);
                }
            }
        }

        loop {
            if has_capacity() {
                schedule_work();
                continue;
            }

            if let Some(node) = finished_task() {
                mark_done(node);
            }

            // If we are all done, break.
            if done() {
                break;
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
            // if something.ready(node) {
            // }
        }*/
    }
}

#[derive(Debug)]
pub struct BuildLog {}

impl BuildLog {}
