extern crate petgraph;

use std::collections::{hash_map::Entry, HashMap};

use petgraph::{graph::NodeIndex, visit::DfsPostOrder, Direction};

// use ninja_desc::{BuildGraph, TaskResult, TasksMap};
use ninja_interface::*;
use ninja_tasks::{Key, Tasks};

mod rebuilder;
mod task;
pub use rebuilder::{MTimeRebuilder, MTimeState};

// Needs to be public for some weird reason.
// This genericity is getting very wonky.
#[derive(Debug)]
pub struct TaskResult {}

type CompatibleRebuilder<'a, State> = &'a dyn Rebuilder<Key, TaskResult, State>;

type SchedulerGraph<'a> = petgraph::Graph<&'a Key, ()>;

#[derive(Debug)]
pub struct ParallelTopoScheduler<State> {
    _unused: std::marker::PhantomData<State>,
}

impl<State> ParallelTopoScheduler<State> {
    pub fn new() -> Self {
        ParallelTopoScheduler {
            _unused: std::marker::PhantomData::default(),
        }
    }

    fn build_graph(tasks: &Tasks) -> SchedulerGraph {
        let mut keys_to_nodes: HashMap<&Key, NodeIndex> = HashMap::new();
        let mut graph = SchedulerGraph::new();
        fn add_or_get_node<'a>(
            map: &mut HashMap<&'a Key, NodeIndex>,
            graph: &mut SchedulerGraph<'a>,
            key: &'a Key,
        ) -> NodeIndex {
            match map.entry(key) {
                Entry::Vacant(e) => {
                    let node = graph.add_node(key);
                    e.insert(node);
                    node
                }
                Entry::Occupied(e) => *e.get(),
            }
        }
        for (key, task) in tasks.all_tasks() {
            let source = add_or_get_node(&mut keys_to_nodes, &mut graph, key);
            for dep in task.dependencies() {
                let dep_node = add_or_get_node(&mut keys_to_nodes, &mut graph, dep);
                graph.add_edge(source, dep_node, ());
            }
        }
        graph
    }

    fn schedule_internal(
        &self,
        rebuilder: CompatibleRebuilder<State>,
        mut state: State,
        tasks: &Tasks,
        start: Option<Vec<Key>>,
    ) {
        assert!(start.is_none(), "not implemented non-externals yet");
        // TODO: Ok we can finally build a graph here.
        let graph = Self::build_graph(&tasks);

        // Cannot use depth_first_search which doesn't say if it is postorder.
        // Cannot use Topo since it doesn't offer move_to and partial traversals.
        // TODO: So we really need to enforce no cycles here.
        let mut visitor = DfsPostOrder::empty(&graph);
        let mut build_order = Vec::new();
        let externals = graph.externals(Direction::Incoming);
        for start in externals {
            let key = graph[start];
            let path = tasks.path_for(key);
            visitor.move_to(start);
            while let Some(node) = visitor.next(&graph) {
                // TODO: Do we really need this list?
                // Seems like what we want is a PQ or something where things in earlier in the
                // topo-sort show up first and then we peek and only pop if they are ready to be
                // built or something.
                // specifically, even though this DFS is useful to find the first few nodes
                // to schedule, and calculates the topo-sort of the _reachable_ nodes, we can't
                // actually act on that info beyond this point. Instead we need to just watch tasks
                // finish, find their dependants and do a check for them.
                build_order.push(node);
                // if self.graph.edges_directed(node, Direction::Outgoing).count() == 0 {
                //     build_state.ready.push_back(node);
                // }
            }
        }

        for node in build_order {
            // TODO: Parts of the interface that don't comply:
            // being able to get dependencies from a task instead of going back to the key. having
            // this notion of a store "context" in which this operates, so that the
            let key = graph[node];
            if let Some(task) = tasks.task(key) {
                if task.is_command() {
                    let build_task = rebuilder.build(key.clone(), TaskResult {}, task);
                    build_task.run(&mut state);
                }
            }
        }
    }
}

impl<State> Scheduler<Key, TaskResult, State> for ParallelTopoScheduler<State> {
    fn schedule(
        &self,
        _rebuilder: CompatibleRebuilder<State>,
        _state: State,
        _tasks: &Tasks,
        _start: Vec<Key>,
    ) {
        todo!("Not implemented");
    }

    fn schedule_externals(
        &self,
        rebuilder: CompatibleRebuilder<State>,
        state: State,
        tasks: &Tasks,
    ) {
        self.schedule_internal(rebuilder, state, tasks, None);
    }
}

pub fn build_externals<K, V, State>(
    scheduler: impl Scheduler<K, V, State>,
    rebuilder: impl Rebuilder<K, V, State>,
    tasks: &Tasks,
    state: State,
) {
    &scheduler.schedule_externals(&rebuilder, state, tasks);
}
