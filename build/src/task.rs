use std::process::{Command, Output};

use thiserror::Error;

use crate::interface::BuildTask;

use crate::TaskResult;

pub trait ParallelTopoTask<State>: BuildTask<State, TaskResult>
where
    State: Sync,
{
}

#[derive(Error, Debug)]
pub enum CommandTaskError {
    #[error("{0}")]
    SpawnFailed(#[from] std::io::Error),
    #[error("failed with {}", .0.status)]
    CommandFailed(Output),
}

pub type CommandTaskResult = Result<Output, CommandTaskError>;

#[derive(Debug)]
pub struct CommandTask {
    command: String,
}

impl CommandTask {
    pub fn new(command: String) -> CommandTask {
        CommandTask { command }
    }

    pub fn run_command(&self) -> CommandTaskResult {
        eprintln!("{}", &self.command);
        let output = Command::new("/bin/sh")
            .arg("-c")
            .arg(&self.command)
            .output()?;
        if !output.status.success() {
            return Err(CommandTaskError::CommandFailed(output));
        }
        Ok(output)
    }
}

impl BuildTask<(), TaskResult> for CommandTask {
    fn run(&self, _state: &()) -> TaskResult {
        self.run_command()
    }

    #[cfg(test)]
    fn is_command(&self) -> bool {
        true
    }
}
