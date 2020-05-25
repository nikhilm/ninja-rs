extern crate petgraph;

use crossbeam::{deque::Steal, scope};
use std::{
    collections::{hash_map::Entry, HashMap, HashSet, VecDeque},
    sync::atomic::Ordering,
};

use petgraph::{graph::NodeIndex, visit::DfsPostOrder, Direction};

// use ninja_desc::{BuildGraph, TaskResult, TasksMap};
use ninja_interface::*;
use ninja_tasks::{Key, Tasks};

pub mod disk_interface;
mod rebuilder;
mod task;
pub use rebuilder::{MTimeRebuilder, MTimeState};

// Needs to be public for some weird reason.
// This genericity is getting very wonky.
#[derive(Debug)]
pub struct TaskResult {}

type CompatibleRebuilder<'a, State> = &'a (dyn Rebuilder<Key, TaskResult, State>);
type CompatibleBuildTask<State> = Box<dyn BuildTask<State, TaskResult> + Send>;

type SchedulerGraph<'a> = petgraph::Graph<&'a Key, ()>;

#[derive(Debug)]
pub struct ParallelTopoScheduler<State> {
    _unused: std::marker::PhantomData<State>,
}

enum QueueTask<State> {
    Stop,
    Task((NodeIndex, CompatibleBuildTask<State>)),
}

impl<State> ParallelTopoScheduler<State>
where
    State: Sync,
{
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
        state: State,
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
        let mut ready = VecDeque::new();
        let mut waiting_tasks = HashSet::new();
        let externals = graph.externals(Direction::Incoming);
        let mut wanted = 0;
        for start in externals {
            let key = graph[start];
            visitor.move_to(start);
            while let Some(node) = visitor.next(&graph) {
                wanted += 1;
                // TODO: Do we really need this list?
                // Seems like what we want is a PQ or something where things in earlier in the
                // topo-sort show up first and then we peek and only pop if they are ready to be
                // built or something.
                // specifically, even though this DFS is useful to find the first few nodes
                // to schedule, and calculates the topo-sort of the _reachable_ nodes, we can't
                // actually act on that info beyond this point. Instead we need to just watch tasks
                // finish, find their dependants and do a check for them.
                if graph.edges_directed(node, Direction::Outgoing).count() == 0 {
                    ready.push_back(node);
                } else {
                    waiting_tasks.insert(node);
                }
            }
        }

        let mut finished = HashSet::new();

        let state_ref = &state;

        let finish_node = |node,
                           finished: &mut HashSet<NodeIndex>,
                           ready: &mut VecDeque<NodeIndex>,
                           waiting_tasks: &mut HashSet<NodeIndex>| {
            finished.insert(node);
            // See if this freed up any pending tasks to run.
            for dependent in graph.neighbors_directed(node, Direction::Incoming) {
                if !waiting_tasks.contains(&dependent) {
                    continue;
                }
                if graph
                    .neighbors_directed(dependent, Direction::Outgoing)
                    .all(|dependency| finished.contains(&dependency))
                {
                    waiting_tasks.remove(&dependent);
                    ready.push_back(dependent);
                }
            }
        };

        const CAP: usize = 8;
        let (tx, rx) = std::sync::mpsc::sync_channel(CAP);
        // Non-stealing queue.
        let job_deque: crossbeam::deque::Injector<QueueTask<State>> =
            crossbeam::deque::Injector::new();
        let running_jobs = std::sync::atomic::AtomicUsize::new(0);

        scope(|s| {
            for _ in 0..CAP {
                // handles will be collected by the scope.
                s.spawn(|_| loop {
                    if let Steal::Success(task) = job_deque.steal() {
                        match task {
                            QueueTask::Stop => break,
                            QueueTask::Task((node, task)) => {
                                running_jobs.fetch_add(1, Ordering::SeqCst);
                                let result = task.run(state_ref);
                                // TODO: This unwrap can make the job count end up wrong.
                                tx.send((node, result)).unwrap();
                                running_jobs.fetch_sub(1, Ordering::SeqCst);
                            }
                        }
                    }
                });
            }

            while finished.len() < wanted {
                // Inherently racy.
                if running_jobs.load(Ordering::Relaxed) < CAP {
                    // we have capacity.
                    if let Some(node) = ready.pop_front() {
                        let key = graph[node];
                        if let Some(task) = tasks.task(key) {
                            let build_task = rebuilder.build(key.clone(), TaskResult {}, task);
                            job_deque.push(QueueTask::Task((node, build_task)));
                        } else {
                            finish_node(node, &mut finished, &mut ready, &mut waiting_tasks);
                        }
                        // we were able to queue a task, so go back to the start of the loop.
                        continue;
                    }
                }

                // we are either at full capacity, or can't make any progress on ready tasks, so
                // wait for something to be done.
                let (node, _result) = rx.recv().unwrap();
                // This will update ready and finished, so we will have made progress.
                finish_node(node, &mut finished, &mut ready, &mut waiting_tasks);
            }

            for _ in 0..CAP {
                job_deque.push(QueueTask::Stop);
            }
        })
        .unwrap();
    }
}

impl<State> Scheduler<Key, TaskResult, State> for ParallelTopoScheduler<State>
where
    State: Sync,
{
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
) where
    State: Sync,
{
    &scheduler.schedule_externals(&rebuilder, state, tasks);
}
