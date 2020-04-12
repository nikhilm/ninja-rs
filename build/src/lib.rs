extern crate ninja_desc;

// TODO: Should eventually move to a concrete implementation of the task abstraction.
use std::{ffi::OsStr, os::unix::ffi::OsStrExt, process::Command, rc::Rc};

use ninja_desc::{BuildDescription, NodeIndex};

#[derive(Debug)]
pub struct BuildState {}

#[derive(Debug)]
pub struct Scheduler {
    desc: Rc<BuildDescription>,
    state: BuildState,
    rebuilder: Rebuilder,
}

impl Scheduler {
    pub fn new(desc: Rc<BuildDescription>, state: BuildState, rebuilder: Rebuilder) -> Scheduler {
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
        for idx in self.desc.nodes_with_no_incoming_edges() {
            self.rebuilder.build(idx);
        }
    }
}

#[derive(Debug)]
pub struct Rebuilder {
    desc: Rc<BuildDescription>,
}

impl Rebuilder {
    pub fn new(desc: Rc<BuildDescription>) -> Rebuilder {
        Rebuilder { desc: desc }
    }

    // this maps roughly to requesting a rebuilder to bring the keys "outputs" for an edge up
    // to date if required by running the edge's command. It can be hidden behind some more
    // abstraction to move the rebuilder away from the "run a command" to "execute a task"
    // paradigm.
    fn build(&self, target: NodeIndex) {
        // actually the rebuilder needs to walk the edges to determine if deps are already up to
        // date.

        // Bring dependencies up to date.
        for dep in self.desc.direct_dependencies(target) {
            self.build(dep);
        }

        // Run command.
        let command = self.desc.command(target);
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
}

#[derive(Debug)]
pub struct BuildLog {}

impl BuildLog {
    pub fn read() -> BuildState {
        BuildState {}
    }
}
