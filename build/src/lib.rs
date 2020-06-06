extern crate petgraph;

use std::collections::{hash_map::Entry, HashMap, HashSet, VecDeque};
use thiserror::Error;

use petgraph::{graph::NodeIndex, visit::DfsPostOrder, Direction};

// use ninja_desc::{BuildGraph, TaskResult, TasksMap};
use ninja_tasks::{Key, Tasks};

mod command_pool;
pub mod disk_interface;
mod interface;
mod rebuilder;
mod task;

use command_pool::{CommandPool, CommandPoolTask};
use disk_interface::SystemDiskInterface;
use interface::*;
pub use rebuilder::{MTimeRebuilder, MTimeState, RebuilderError};
use task::{CommandTaskError, CommandTaskResult};

// Needs to be public for some weird reason.
// This genericity is getting very wonky.
type TaskResult = CommandTaskResult;

type CompatibleRebuilder<'a, State> = &'a (dyn Rebuilder<Key, TaskResult, State, RebuilderError>);
type CompatibleBuildTask<State> = Box<dyn BuildTask<State, TaskResult> + Send>;

type SchedulerGraph<'a> = petgraph::Graph<&'a Key, ()>;

#[derive(Error, Debug)]
pub enum BuildError {
    #[error("{0}")]
    RebuilderError(#[from] RebuilderError),
    #[error("command pool panic")]
    CommandPoolPanic,
    #[error("command failed {0}")]
    CommandFailed(#[from] CommandTaskError),
}

struct CommandPoolWrapperTask<'a, State>
where
    State: Sync,
{
    node: NodeIndex,
    task: CompatibleBuildTask<State>,
    state_ref: &'a State,
}

impl<'a, State> CommandPoolWrapperTask<'a, State>
where
    State: Sync,
{
    pub fn new(node: NodeIndex, task: CompatibleBuildTask<State>, state_ref: &'a State) -> Self {
        CommandPoolWrapperTask {
            node,
            task,
            state_ref,
        }
    }
}
impl<'a, State> CommandPoolTask for CommandPoolWrapperTask<'a, State>
where
    State: Sync,
{
    type Result = (NodeIndex, TaskResult);

    fn run(&self) -> Self::Result {
        (self.node, self.task.run(self.state_ref))
    }
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
    ) -> Result<(), BuildError> {
        assert!(start.is_none(), "not implemented non-externals yet");
        let graph = Self::build_graph(&tasks);
        let mut build_state = BuildState::default();

        // Cannot use depth_first_search which doesn't say if it is postorder.
        // Cannot use Topo since it doesn't offer move_to and partial traversals.
        // TODO: So we really need to enforce no cycles here.
        let mut visitor = DfsPostOrder::empty(&graph);
        let externals = graph.externals(Direction::Incoming);
        for start in externals {
            visitor.move_to(start);
            while let Some(node) = visitor.next(&graph) {
                build_state.add_node(&graph, node);
            }
        }

        let state_ref = &state;
        let command_pool = CommandPool::new();
        command_pool
            .run(|scope| -> Result<(), BuildError> {
                while !build_state.done() {
                    // Inherently racy.
                    if scope.has_capacity() {
                        // we have capacity.
                        if let Some(node) = build_state.next_ready() {
                            let key = graph[node];
                            if let Some(task) = tasks.task(key) {
                                // TODO: handle error
                                let rebuilder_result = rebuilder.build(key.clone(), task);
                                if let Err(e) = rebuilder_result {
                                    return Err(From::from(e));
                                }
                                let build_task = rebuilder_result.unwrap();
                                if let Some(build_task) = build_task {
                                    scope.enqueue(CommandPoolWrapperTask::new(
                                        node, build_task, state_ref,
                                    ));
                                } else {
                                    // Phony or something. Always succeeds.
                                    build_state.finish_node(&graph, node, true);
                                }
                            } else {
                                // No task, so this is a source and we are done.
                                build_state.finish_node(&graph, node, true);
                            }
                            // One of N things happened.
                            // We clearly had capacity, and we were able to find a ready task.
                            // This means we "made progress", either enqueuing the task or
                            // immediately marking it as done. So try to do more queueing.
                            continue;
                        }
                        // We have capacity, but no ready task to run, so just wait for existing
                        // tasks to make progress.
                    }

                    // we are either at full capacity, or waiting for tasks to finish so more tasks
                    // can be ready, so wait for progress.
                    let (node, result) = scope.rx.recv().unwrap();
                    // Hmm... need a way to convey result to the outside world later, but keep going with
                    // other tasks. In addition, don't want to pretend something is wrong with the
                    // queue itself.
                    if let Err(e) = result {
                        // This will update ready and finished, so we will have made progress.
                        build_state.finish_node(&graph, node, false);
                        eprintln!("{}", e);
                        match e {
                            CommandTaskError::CommandFailed(output) => {
                                eprintln!("{}", String::from_utf8(output.stderr).unwrap());
                            }
                            _ => {}
                        }
                    } else {
                        // This will update ready and finished, so we will have made progress.
                        build_state.finish_node(&graph, node, true);
                    }
                }
                Ok(())
            })
            .map_err(|_| BuildError::CommandPoolPanic)
            .and_then(|r| r)
    }
}

#[derive(Debug, Default)]
struct BuildState {
    wanted: usize,
    finished: HashSet<NodeIndex>,
    ready: VecDeque<NodeIndex>,
    waiting_tasks: HashSet<NodeIndex>,
}

impl BuildState {
    pub fn done(&self) -> bool {
        assert!(self.finished.len() <= self.wanted);
        self.finished.len() == self.wanted
    }

    pub fn next_ready(&mut self) -> Option<NodeIndex> {
        assert!(!self.done());
        self.ready.pop_front()
    }

    pub fn add_node(&mut self, graph: &SchedulerGraph, node: NodeIndex) {
        self.wanted += 1;
        if graph.edges_directed(node, Direction::Outgoing).count() == 0 {
            // No dependencies, we can start this immediately.
            self.ready.push_back(node);
        } else {
            // Has dependencies, wait until they are done.
            self.waiting_tasks.insert(node);
        }
    }

    fn finish_node_success(&mut self, graph: &SchedulerGraph, node: NodeIndex) {
        // See if this freed up any pending tasks to run.
        for dependent in graph.neighbors_directed(node, Direction::Incoming) {
            if !self.waiting_tasks.contains(&dependent) {
                // This dependent must've already failed due to another dependency.
                // TODO: Wish we could assert it has failed.
                debug_assert!(self.finished.contains(&dependent));
                continue;
            }
            debug_assert!(!self.finished.contains(&dependent));
            if graph
                .neighbors_directed(dependent, Direction::Outgoing)
                .all(|dependency| self.finished.contains(&dependency))
            {
                self.waiting_tasks.remove(&dependent);
                self.ready.push_back(dependent);
            }
        }
    }

    /*
     *             (A) [running]   (B) [fails]
     *               \    /
     *                 (C) [waiting] -> [finished]
     */

    fn finish_node_error(&mut self, graph: &SchedulerGraph, node: NodeIndex) {
        for dependent in graph.neighbors_directed(node, Direction::Incoming) {
            if !self.waiting_tasks.contains(&dependent) {
                debug_assert!(self.finished.contains(&dependent));
                continue;
            }
            debug_assert!(!self.finished.contains(&dependent));
            self.waiting_tasks.remove(&dependent);
            self.finished.insert(dependent);
            // Recursively fail all tasks.
            self.finish_node_error(graph, dependent);
        }
    }

    pub fn finish_node(&mut self, graph: &SchedulerGraph, node: NodeIndex, succeeded: bool) {
        // Mark the task as finished regardless of failure.
        self.finished.insert(node);

        // See if any further tasks can be kicked off.
        if succeeded {
            self.finish_node_success(graph, node);
        } else {
            // OK. We want to make sure tasks that depend on this do not run (recursively), but
            // we still make progress.
            // We could mark dependents as done. We can assert that they are not in the ready
            // queue already. We can assert they are in the waiting queue. Then remove them
            // from waiting.
            // What do we mark them finished as? i.e. if we mark as success, dependents will be
            // queued up and run commands. We specifically want to fail them all.
            self.finish_node_error(graph, node);
        }
    }
}

#[derive(Debug)]
pub struct ParallelTopoScheduler<State> {
    _unused: std::marker::PhantomData<State>,
}

impl<State> Scheduler<Key, TaskResult, State, BuildError, RebuilderError>
    for ParallelTopoScheduler<State>
where
    State: Sync,
{
    fn schedule(
        &self,
        _rebuilder: CompatibleRebuilder<State>,
        _state: State,
        _tasks: &Tasks,
        _start: Vec<Key>,
    ) -> Result<(), BuildError> {
        todo!("Not implemented");
    }

    fn schedule_externals(
        &self,
        rebuilder: CompatibleRebuilder<State>,
        state: State,
        tasks: &Tasks,
    ) -> Result<(), BuildError> {
        self.schedule_internal(rebuilder, state, tasks, None)
    }
}

pub fn build_externals<K, V, State>(
    scheduler: impl Scheduler<K, V, State, BuildError, RebuilderError>,
    rebuilder: impl Rebuilder<K, V, State, RebuilderError>,
    tasks: &Tasks,
    state: State,
) -> Result<(), BuildError>
where
    State: Sync,
{
    Ok(scheduler.schedule_externals(&rebuilder, state, tasks)?)
}

pub fn default_mtimestate() -> MTimeState<SystemDiskInterface> {
    MTimeState::new(SystemDiskInterface {})
}
