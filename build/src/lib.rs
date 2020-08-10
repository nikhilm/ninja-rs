/*
 * Copyright 2020 Nikhil Marathe <nsm.nikhil@gmail.com>
 *
 * Licensed under the Apache License, Version 2.0 (the "License");
 * you may not use this file except in compliance with the License.
 * You may obtain a copy of the License at
 *
 *     http://www.apache.org/licenses/LICENSE-2.0
 *
 * Unless required by applicable law or agreed to in writing, software
 * distributed under the License is distributed on an "AS IS" BASIS,
 * WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
 * See the License for the specific language governing permissions and
 * limitations under the License.
 */

#![feature(option_expect_none)]

extern crate petgraph;

use std::io::Write;
use tokio::{runtime::Builder, sync::Semaphore, task::LocalSet};

use std::collections::{hash_map::Entry, HashMap, HashSet, VecDeque};
use thiserror::Error;

use petgraph::{graph::NodeIndex, visit::DfsPostOrder, Direction};

pub mod disk_interface;
mod interface;
mod rebuilder;
pub mod task;
use task::{Key, Task, Tasks};

#[cfg(test)]
mod property_tests;

use disk_interface::SystemDiskInterface;
pub use rebuilder::{MTimeRebuilder, MTimeState, RebuilderError};
use task::{CommandTaskError, CommandTaskResult};

// Needs to be public for some weird reason.
// This genericity is getting very wonky.
type TaskResult = CommandTaskResult;

type SchedulerGraph<'a> = petgraph::Graph<&'a Key, ()>;

#[derive(Error, Debug)]
pub enum BuildError {
    #[error("command pool panic")]
    CommandPoolPanic,
    #[error("command failed {0}")]
    CommandFailed(#[from] CommandTaskError),
    #[error(transparent)]
    RebuilderError(#[from] anyhow::Error),
}

#[derive(Debug)]
struct Printer {
    finished: usize,
    total: usize,
    console: console::Term,
}

impl Default for Printer {
    fn default() -> Self {
        Printer {
            finished: 0,
            total: 0,
            console: console::Term::stdout(),
        }
    }
}

impl Printer {
    fn print_status(&mut self, task: &Task) {
        let command = task.command().expect("only command tasks");

        if self.console.is_term() {
            self.console.clear_line().expect("clear");
            write!(
                self.console,
                "[{}/{}] {}",
                self.finished, self.total, command
            )
            .expect("write");
        } else {
            writeln!(
                self.console,
                "[{}/{}] {}",
                self.finished, self.total, command
            )
            .expect("write");
        }
    }

    fn started(&mut self, task: &Task) {
        self.total += 1;
        self.print_status(task);
    }

    fn finished(&mut self, task: &Task, result: CommandTaskResult) {
        self.finished += 1;
        self.print_status(task);
        if let Ok(output) = result {
            if !output.stdout.is_empty() {
                write!(
                    self.console,
                    "\n{}", // TODO: Correct newline handling.
                    std::str::from_utf8(&output.stdout).unwrap()
                )
                .unwrap();
            }
        } else {
            // TODO: Print build edge.
            writeln!(self.console, "\nFAILED\n{}", task.command().unwrap()).unwrap();
            match result.unwrap_err() {
                err @ CommandTaskError::SpawnFailed(_) => {
                    writeln!(self.console, "Failed to spawn command: {}", err).unwrap();
                }
                CommandTaskError::CommandFailed(out) => {
                    // ninja interleaves streams, but this will do for now.
                    self.console.write(&out.stdout).unwrap();
                    self.console.write(&out.stderr).unwrap();
                }
            }
        }
    }
}

impl Drop for Printer {
    fn drop(&mut self) {
        // For now, print a final newline since our status printer isn't.
        if self.console.is_term() {
            self.console.write_line("").unwrap();
        }
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
pub struct ParallelTopoScheduler {
    parallelism: usize,
}

impl ParallelTopoScheduler
{
    pub fn new(parallelism: usize) -> Self {
        ParallelTopoScheduler {
            parallelism,
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
            for dep in task.dependencies().iter().chain(task.order_dependencies()) {
                let dep_node = add_or_get_node(&mut keys_to_nodes, &mut graph, dep);
                graph.add_edge(source, dep_node, ());
            }
        }
        graph
    }

    fn schedule_internal(
        &self,
        rebuilder: &impl interface::Rebuilder<Key, TaskResult>,
        tasks: &Tasks,
        start: Option<Vec<Key>>,
    ) -> Result<(), BuildError> {
        let graph = Self::build_graph(&tasks);
        let mut build_state = BuildState::default();
        let mut printer = Printer::default();

        // Cannot use depth_first_search which doesn't say if it is postorder.
        // Cannot use Topo since it doesn't offer move_to and partial traversals.
        // TODO: So we really need to enforce no cycles here.
        let mut visitor = DfsPostOrder::empty(&graph);
        let requested: Box<dyn Iterator<Item = NodeIndex>> = match start {
            Some(keys) => {
                let x = &graph;
                Box::new(
                    graph
                        .node_indices()
                        .filter(move |idx| keys.contains(x[*idx])),
                )
            }
            None => Box::new(graph.externals(Direction::Incoming)),
        };
        for start in requested {
            visitor.move_to(start);
            while let Some(node) = visitor.next(&graph) {
                build_state.add_node(&graph, node);
            }
        }

        let local_set = LocalSet::new();
        let mut runtime = Builder::new()
            .enable_all()
            .basic_scheduler()
            .enable_all()
            .build()
            .unwrap();

        let mut pending = Vec::new();
        let sem = Semaphore::new(self.parallelism);
        local_set.block_on(&mut runtime, async {
            while !build_state.done() {
                if let Some(node) = build_state.next_ready() {
                    let key = graph[node];
                    if let Some(task) = tasks.task(key) {
                        // TODO: handle error
                        let rebuilder_result = rebuilder.build(key.clone(), None, task);
                        if let Err(e) = rebuilder_result {
                            // TODO: convey the real error.
                            return Err(From::from(anyhow::Error::new(e)));
                        }
                        let build_task = rebuilder_result.unwrap();
                        printer.started(task);
                        let sem = &sem;
                        pending.push(Box::pin(async move {
                            let _p = sem.acquire().await;
                            futures::future::ready((node, build_task.run().await)).await
                        }));
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

                let (finished, _, left) = futures::future::select_all(pending).await;
                pending = left;

                let (node, result) = finished;
                // Hmm... need a way to convey result to the outside world later, but keep going with
                // other tasks. In addition, don't want to pretend something is wrong with the
                // queue itself.
                // This will update ready and finished, so we will have made progress.
                build_state.finish_node(&graph, node, result.is_ok());

                // If we executed something, that node must have a key and task.
                let key = graph[node];
                let task = tasks.task(key);
                printer.finished(task.unwrap(), result);
            }
            assert!(pending.is_empty());
            Ok(())
        })
    }
}

impl interface::Scheduler<Key, TaskResult> for ParallelTopoScheduler
{
    type Error = BuildError;

    fn schedule(
        &self,
        rebuilder: &impl interface::Rebuilder<Key, TaskResult>,
        tasks: &Tasks,
        start: Vec<Key>,
    ) -> Result<(), Self::Error> {
        self.schedule_internal(rebuilder,  tasks, Some(start))
    }

    fn schedule_externals(
        &self,
        rebuilder: &impl interface::Rebuilder<Key, TaskResult>,
        tasks: &Tasks,
    ) -> Result<(), Self::Error> {
        self.schedule_internal(rebuilder,  tasks, None)
    }
}

pub fn build_externals<K, V, Scheduler>(
    scheduler: Scheduler,
    rebuilder: &impl interface::Rebuilder<K, V>,
    tasks: &Tasks,
) -> Result<(), Scheduler::Error>
where
    Scheduler: interface::Scheduler<K, V>
{
    Ok(scheduler.schedule_externals(rebuilder, tasks)?)
}

pub fn build<K, V, Scheduler>(
    scheduler: Scheduler,
    rebuilder: &impl interface::Rebuilder<K, V>,
    tasks: &Tasks,
    start: Vec<K>,
) -> Result<(), Scheduler::Error>
where
    Scheduler: interface::Scheduler<K, V>
{
    Ok(scheduler.schedule(rebuilder, tasks, start)?)
}

pub fn default_mtimestate() -> MTimeState<SystemDiskInterface> {
    MTimeState::new(SystemDiskInterface {})
}
