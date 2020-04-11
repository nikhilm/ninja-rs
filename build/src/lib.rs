extern crate ninja_desc;

use ninja_desc::BuildDescription;

pub struct BuildState {}

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

    pub fn run(self) {}
}

pub struct Rebuilder {}

impl Rebuilder {
    pub fn new() -> Rebuilder {
        Rebuilder {}
    }
}

pub struct BuildLog {}

impl BuildLog {
    pub fn read() -> BuildState {
        BuildState {}
    }
}
