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

use std::{
    collections::{hash_map::Entry, HashMap, HashSet, VecDeque},
    io::Write,
};

use petgraph::{graph::NodeIndex, visit::DfsPostOrder, Direction};
use thiserror::Error;
use tokio::{sync::Semaphore, task::LocalSet};

mod build_task;
pub mod disk_interface;
pub mod interface;
#[cfg(test)]
mod property_tests;
mod rebuilder;
pub mod task;
pub mod tracking_rebuilder;

use build_task::{CommandTaskError, CommandTaskResult, NinjaTask};
use disk_interface::SystemDiskInterface;
use interface::BuildTask;
pub use rebuilder::{CachingMTimeRebuilder, DiskDirtyCache, RebuilderError};
use task::{Key, Task, Tasks};

type SchedulerGraph<'a> = petgraph::Graph<&'a Key, ()>;

#[derive(Error, Debug)]
pub enum BuildError {
    #[error("command pool panic")]
    CommandPoolPanic,
    #[error("command failed {0}")]
    CommandFailed(#[from] CommandTaskError),
    #[error(transparent)]
    RebuilderError(#[from] Box<dyn std::error::Error + Send + Sync>),
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

// How this is called does need re-doing.
// First, having NoopTask but not passing it the build task means it cannot tell whether a command
// would actually be run or not.
impl Printer {
    fn print_status(&mut self, task: &Task) {
        if !task.is_command() {
            return;
        }
        let command = task.command().unwrap().trim();

        if self.console.is_term() {
            // TODO: Handle non-ASCII properly.
            // TODO: ninja style elision.
            let size = self
                .console
                .size_checked()
                .map(|(_rows, columns)| columns)
                .unwrap_or(80);
            self.console.clear_line().expect("clear");
            write!(
                self.console,
                "[{}/{}] {}",
                // TODO: Properly calculate instead of just removing 10 chars.
                self.finished,
                self.total,
                &command[..std::cmp::min(command.len(), (size as usize) - 10)]
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
            panic!("FAILED");
        }
    }
}

impl Drop for Printer {
    fn drop(&mut self) {
        if self.console.is_term() {
            if self.total > 0 {
                self.console.write_line("").unwrap();
            } else {
                self.console.write_line("ninja: no work to do.").unwrap();
            }
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

impl ParallelTopoScheduler {
    pub fn new(parallelism: usize) -> Self {
        ParallelTopoScheduler { parallelism }
    }

    fn build_graph(tasks: &Tasks, start: Option<Vec<Key>>) -> SchedulerGraph {
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

        let task_map = tasks.all_tasks();

        if let Some(start) = start {
            // The borrow checker has a problem with recursion, so bring out the BFS.
            let mut queue = std::collections::VecDeque::from(start);
            let mut visited = HashSet::new();
            while !queue.is_empty() {
                let key = queue.pop_front().unwrap();
                if let Some((key, task)) = task_map.get_key_value(&key) {
                    let source = add_or_get_node(&mut keys_to_nodes, &mut graph, key);
                    if !visited.contains(&source) {
                        visited.insert(source);
                        for dep in task.dependencies().iter().chain(task.order_dependencies()) {
                            let dep_node = add_or_get_node(&mut keys_to_nodes, &mut graph, dep);
                            graph.add_edge(source, dep_node, ());
                            queue.push_back(dep.clone());
                        }
                    }
                }
            }
        } else {
            for (key, task) in task_map {
                let source = add_or_get_node(&mut keys_to_nodes, &mut graph, key);
                for dep in task.dependencies().iter().chain(task.order_dependencies()) {
                    let dep_node = add_or_get_node(&mut keys_to_nodes, &mut graph, dep);
                    graph.add_edge(source, dep_node, ());
                }
            }
        }
        graph
    }

    fn schedule_internal(
        &self,
        rebuilder: &impl interface::Rebuilder<Key, CommandTaskResult>,
        tasks: &Tasks,
        start: Option<Vec<Key>>,
    ) -> Result<(), BuildError> {
        // Umm.. OK So if the user did not request a particular start, and there are no defaults,
        // then we need to first build a graph and then find the externals.
        // But if there is a start, could we build a graph that has only reachable nodes, and also
        // get our topo sort at the same time?
        let graph = Self::build_graph(&tasks, start.clone());
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
        let mut runtime = tokio::runtime::Builder::new()
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
                        if let Some(build_task) = rebuilder
                            .build(key.clone(), None, task)
                            .map_err(|e| BuildError::RebuilderError(Box::new(e)))?
                        {
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

impl interface::Scheduler<Key, CommandTaskResult> for ParallelTopoScheduler {
    type Error = BuildError;

    fn schedule(
        &self,
        rebuilder: &impl interface::Rebuilder<Key, CommandTaskResult>,
        tasks: &Tasks,
        start: Vec<Key>,
    ) -> Result<(), Self::Error> {
        self.schedule_internal(rebuilder, tasks, Some(start))
    }

    fn schedule_externals(
        &self,
        rebuilder: &impl interface::Rebuilder<Key, CommandTaskResult>,
        tasks: &Tasks,
    ) -> Result<(), Self::Error> {
        self.schedule_internal(rebuilder, tasks, None)
    }
}

pub fn build_externals<K, V, Scheduler>(
    scheduler: &Scheduler,
    rebuilder: &impl interface::Rebuilder<K, V>,
    tasks: &Tasks,
) -> Result<(), Scheduler::Error>
where
    Scheduler: interface::Scheduler<K, V>,
{
    Ok(scheduler.schedule_externals(rebuilder, tasks)?)
}

pub fn build<K, V, Scheduler>(
    scheduler: &Scheduler,
    rebuilder: &impl interface::Rebuilder<K, V>,
    tasks: &Tasks,
    start: Vec<K>,
) -> Result<(), Scheduler::Error>
where
    Scheduler: interface::Scheduler<K, V>,
{
    Ok(scheduler.schedule(rebuilder, tasks, start)?)
}

pub fn caching_mtime_rebuilder() -> CachingMTimeRebuilder<DiskDirtyCache<SystemDiskInterface>> {
    CachingMTimeRebuilder::new(DiskDirtyCache::new(SystemDiskInterface {}))
}
