use std::process::Output;
use tokio::process::Command;

use thiserror::Error;

use crate::interface::BuildTask;

use crate::TaskResult;

use async_trait::async_trait;

pub trait ParallelTopoTask: BuildTask<TaskResult>
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

    pub async fn run_command(&self) -> CommandTaskResult {
        eprintln!("{}", &self.command);
        let output = Command::new("/bin/sh")
            .arg("-c")
            .arg(&self.command)
            .output()
            .await?;
        if !output.status.success() {
            return Err(CommandTaskError::CommandFailed(output));
        }
        Ok(output)
    }
}

#[async_trait(?Send)]
impl BuildTask<TaskResult> for CommandTask {
    async fn run(&self) -> TaskResult {
            self.run_command().await
    }

    #[cfg(test)]
    fn is_command(&self) -> bool {
        true
    }
}
