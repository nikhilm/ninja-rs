extern crate ninja_desc;
extern crate ninja_interface;
extern crate petgraph;

use std::collections::{HashMap, HashSet, VecDeque};

use petgraph::{graph::NodeIndex, visit::DfsPostOrder, Direction};

use ninja_desc::{BuildGraph, TaskResult, TasksMap};
use ninja_interface::{Rebuilder, Scheduler, Task};

mod rebuilder;
pub use rebuilder::MTimeRebuilder;

#[derive(Debug, Default)]
struct BuildState {
    ready: VecDeque<NodeIndex>,
    finished: HashMap<NodeIndex, TaskResult>,
}

type MyRebuilder<'a> = &'a dyn Rebuilder<NodeIndex, TaskResult>;

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
    fn schedule<'b>(&self, rebuilder: MyRebuilder<'b>, start: Vec<NodeIndex>) {
        let mut build_state: BuildState = Default::default();
        // Cannot use depth_first_search which doesn't say if it is postorder.
        // Cannot use Topo since it doesn't offer move_to and partial traversals.
        // TODO: So we really need to enforce no cycles here.
        let mut visitor = DfsPostOrder::empty(self.graph);
        let mut build_order = Vec::new();
        for start in start {
            visitor.move_to(start);
            while let Some(node) = visitor.next(self.graph) {
                // TODO: Do we really need this list?
                // Seems like what we want is a PQ or something where things in earlier in the
                // topo-sort show up first and then we peek and only pop if they are ready to be
                // built or something.
                // specifically, even though this DFS is useful to find the first few nodes
                // to schedule, and calculates the topo-sort of the _reachable_ nodes, we can't
                // actually act on that info beyond this point. Instead we need to just watch tasks
                // finish, find their dependants and do a check for them.
                build_order.push(node);
                if self.graph.edges_directed(node, Direction::Outgoing).count() == 0 {
                    build_state.ready.push_back(node);
                }
            }
        }
    }
}

#[derive(Debug)]
pub struct BuildLog {}

impl BuildLog {}
