extern crate ninja_desc;
extern crate ninja_interface;
extern crate ninja_paths;
extern crate petgraph;

use std::{
    collections::{HashMap, HashSet, VecDeque},
    sync::mpsc::{sync_channel, Receiver, SyncSender},
    thread::JoinHandle,
};

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

type MyRebuilder<'a> = &'a dyn Rebuilder<NodeIndex, TaskResult>

#[derive(Debug, Default)]
struct RebuildPool {
    capacity: u32,
    running: u32,
    handles: Vec<JoinHandle<_>>,
    sender: SyncSender<_>,
    receiver: Receiver<_>,
}

enum Request {
    Stop,
    Task(NodeIndex, Task<TaskResult>),
}

impl RebuildPool {
    fn new(capacity: u32) -> RebuildPool {
        let (sender, receiver) = sync_channel(0);
        RebuildPool {
            capacity,
            running: 0,
            handles: Vec::new(),
            sender,
            receiver,
        }
    }

    fn has_capacity(&self) -> bool {
        self.running < self.capacity
    }

    fn run_thread(sender: Sender<_>) {
        loop {
            match _get_it {
                Request::Stop => break,
                Request::Task(node, task) => {
                    let result = task.run();
                    sender.send(node, result);
                }
            }
        }
    }

    fn build(&mut self, node: NodeIndex, task: Box<Task<TaskResult>>) {
        // No optimizations for now, simply launch capacity threads.
        if self.handles.is_empty() {
            (0..self.capacity).for_each(|_| {
                let sender = self.sender.clone();
                self.handles.push(std::thread::spawn(move || {
                    run_thread(sender);
                }));
            });
        }
        assert!(self.handles.len() == self.capacity);
        assert!(self.has_capacity());
        self.running += 1;
        // make task available to a thread.
    }

    fn wait_for_task(&mut self) -> (NodeIndex, TaskResult) {
        match self.receiver.recv() {
            Ok(result) => {
                self.running -= 1;
                return result;
            }
            Err(_) => {
                todo!("IMPL");
            }
        }
    }
}

impl Drop for RebuildPool {
    fn drop(&mut self) {
        todo!("IMPL");
    }
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

impl<'a> Scheduler<NodeIndex, TaskResult, PendingProcess> for ParallelTopoScheduler<'a> {
    type RunTask = NinjaTask;
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

        let mut rebuild_pool = RebuildPool::new(8 /* TODO: CPU num */);

        // It isn't exactly clear which parts of the build state belong exclusively to the
        // scheduler, which to the rebuilder and which are shared, and how that affects mutable
        // references/ownership.
        while build_state.finished.len() < build_order.len() {
            // Queue up any ready tasks if we have the capacity.
            // There is no concern of a race here as the state can only change when we drive work.
            if rebuild_pool.has_capacity() && !build_state.ready.is_empty() {
                // Is there some invariant about ready that we can enforce?
                let next = build_state.ready.pop_front().expect("item");
                // TODO: Where to handle missing inputs
                if let Some(task) = self.tasks.get(&next) {
                    let task = rebuilder.build(next, TaskResult {}, task);
                    rebuild_pool.build(next, task);
                }
                continue;
            }

            // If we couldn't queue up anything more, wait until something finishes.
            let (finished, result) = rebuild_pool.wait_for_task();
            // TODO: Mark next task as finished.
            build_state.finished.insert(finished, result);
        }

        rebuild_pool.drop();
    }
}

#[derive(Debug)]
pub struct BuildLog {}

impl BuildLog {}
