use std::process::Command;

use ninja_interface::BuildTask;

use crate::TaskResult;

pub trait ParallelTopoTask<State>: BuildTask<State, TaskResult>
where
    State: Sync,
{
}

#[derive(Debug)]
pub struct NoopTask;
impl BuildTask<(), TaskResult> for NoopTask {
    fn run(&self, _state: &()) -> TaskResult {
        TaskResult {}
    }
}

#[derive(Debug)]
pub struct CommandTask {
    command: String,
}

impl CommandTask {
    pub fn new(command: String) -> CommandTask {
        CommandTask { command }
    }

    pub fn run_command(&self) -> TaskResult {
        eprintln!("{}", &self.command);
        Command::new("/bin/sh")
            .arg("-c")
            .arg(&self.command)
            .status()
            .expect("success");
        // TODO: Handle failure here and throughout the build graph.
        TaskResult {}
    }
}

impl BuildTask<(), TaskResult> for CommandTask {
    fn run(&self, _state: &()) -> TaskResult {
        self.run_command()
    }
}
