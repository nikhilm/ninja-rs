extern crate ninja_desc;

// TODO: Should eventually move to a concrete implementation of the task abstraction.
use std::{cell::RefCell, ffi::OsStr, os::unix::ffi::OsStrExt, process::Command, rc::Rc};

use ninja_desc::{BuildDescription, NodeIndex};

#[derive(Debug)]
pub struct BuildState {}

#[derive(Debug)]
pub struct Scheduler {
    desc: Rc<RefCell<BuildDescription>>,
    state: BuildState,
    rebuilder: Rebuilder,
}

impl Scheduler {
    pub fn new(
        desc: Rc<RefCell<BuildDescription>>,
        state: BuildState,
        rebuilder: Rebuilder,
    ) -> Scheduler {
        Scheduler {
            desc: desc,
            state: state,
            rebuilder: rebuilder,
        }
    }

    pub fn run(mut self) {
        let to_build = {
            let mut desc = self.desc.borrow_mut();
            desc.build_order()
        };
        for idx in to_build {
            self.rebuilder.build(idx);
        }
    }
}

#[derive(Debug)]
pub struct Rebuilder {
    desc: Rc<RefCell<BuildDescription>>,
}

impl Rebuilder {
    pub fn new(desc: Rc<RefCell<BuildDescription>>) -> Rebuilder {
        Rebuilder { desc: desc }
    }

    // this maps roughly to requesting a rebuilder to bring the keys "outputs" for an edge up
    // to date if required by running the edge's command. It can be hidden behind some more
    // abstraction to move the rebuilder away from the "run a command" to "execute a task"
    // paradigm.
    fn build(&self, target: NodeIndex) {
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
    }
}

#[derive(Debug)]
pub struct BuildLog {}

impl BuildLog {
    pub fn read() -> BuildState {
        BuildState {}
    }
}
