extern crate ninja_desc;

// TODO: Should eventually move to a concrete implementation of the task abstraction.
use std::{ffi::OsStr, os::unix::ffi::OsStrExt, process::Command};

use ninja_desc::{BuildDescription, BuildEdge};

#[derive(Debug)]
pub struct BuildState {}

#[derive(Debug)]
pub struct Scheduler {
    desc: BuildDescription,
    state: BuildState,
    rebuilder: Rebuilder,
}

impl Scheduler {
    pub fn new(desc: BuildDescription, state: BuildState, rebuilder: Rebuilder) -> Scheduler {
        Scheduler {
            desc: desc,
            state: state,
            rebuilder: rebuilder,
        }
    }

    pub fn run(mut self) {
        // Really dumb scheduler
        // No topo sort, no "end" finding
        // No dependency resolution. just run one edge.
        for edge in self.desc.edges {
            self.rebuilder.build(edge);
        }
    }
}

#[derive(Debug)]
pub struct Rebuilder {}

impl Rebuilder {
    pub fn new() -> Rebuilder {
        Rebuilder {}
    }

    // this maps roughly to requesting a rebuilder to bring the keys "outputs" for an edge up
    // to date if required by running the edge's command. It can be hidden behind some more
    // abstraction to move the rebuilder away from the "run a command" to "execute a task"
    // paradigm.
    fn build(&mut self, edge: BuildEdge) {
        let command_str: &OsStr = OsStrExt::from_bytes(&edge.command);
        // POSIX only
        Command::new("/bin/sh")
            .arg("-c")
            .arg(command_str)
            .status()
            .expect("success");
    }
}

#[derive(Debug)]
pub struct BuildLog {}

impl BuildLog {
    pub fn read() -> BuildState {
        BuildState {}
    }
}
