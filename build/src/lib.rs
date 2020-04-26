extern crate ninja_desc;
extern crate ninja_interface;

use ninja_interface::{Rebuilder, Scheduler, Task};

// TODO: Should eventually move to a concrete implementation of the task abstraction.
use std::{cell::RefCell, ffi::OsStr, os::unix::ffi::OsStrExt, process::Command, rc::Rc};

use ninja_desc::{BuildGraph, NodeIndex, TaskResult, TasksMap};

#[derive(Debug)]
pub struct BuildState {}

#[derive(Debug)]
pub struct TopoScheduler<'a> {
    graph: &'a BuildGraph,
}

impl<'a> TopoScheduler<'a> {
    pub fn new(graph: &'a BuildGraph) -> TopoScheduler {
        TopoScheduler { graph }
    }
}

impl<'a> Scheduler<NodeIndex, TaskResult> for TopoScheduler<'a> {
    fn schedule(&self, rebuilder: &dyn Rebuilder<NodeIndex, TaskResult>, start: Vec<NodeIndex>) {
        todo!("IMPL");
    }
}

#[derive(Debug)]
pub struct MTimeRebuilder<'a> {
    graph: &'a BuildGraph,
    tasks: TasksMap,
}

impl<'a> MTimeRebuilder<'a> {
    pub fn new(graph: &BuildGraph, tasks: TasksMap) -> MTimeRebuilder {
        MTimeRebuilder { graph, tasks }
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
