use std::{
    os::unix::{ffi::OsStrExt, process::ExitStatusExt},
    process::Output,
};

use async_trait::async_trait;
use thiserror::Error;
use tokio::process::Command;

use crate::task::Key;
use crate::interface::BuildTask;

#[derive(Error, Debug)]
pub enum CommandTaskError {
    #[error("{0}")]
    SpawnFailed(#[from] std::io::Error),
    #[error("failed with {}", .0.status)]
    CommandFailed(Output),
}

pub type CommandTaskResult = Result<Output, CommandTaskError>;

pub trait NinjaTask: BuildTask<CommandTaskResult> + std::fmt::Debug {
    fn is_command(&self) -> bool {
        false
    }
}

#[derive(Debug)]
pub struct CommandTask {
    key: Key,
    command: String,
}

impl CommandTask {
    pub fn new(key: Key, command: String) -> CommandTask {
        CommandTask { key, command }
    }

    pub async fn run_command(&self) -> CommandTaskResult {
        // Create directories for all outputs.
        // TODO: Somehow hide this behind a disk interface or something so we can mock it.
        for output in self.key.iter() {
            if let Some(dir) = std::path::Path::new(std::ffi::OsStr::from_bytes(output.as_bytes())).parent() {
                if !dir.exists() {
                    std::fs::create_dir_all(dir)?;
                }
            }
        }

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
impl BuildTask<CommandTaskResult> for CommandTask {
    async fn run(&self) -> CommandTaskResult {
        self.run_command().await
    }
}

impl NinjaTask for CommandTask {
    #[cfg(test)]
    fn is_command(&self) -> bool {
        true
    }
}

#[derive(Debug, Default)]
pub struct NoopTask {}

#[async_trait(?Send)]
impl BuildTask<CommandTaskResult> for NoopTask {
    async fn run(&self) -> CommandTaskResult {
        futures::future::ready(Ok(std::process::Output {
            status: ExitStatusExt::from_raw(0),
            stdout: vec![],
            stderr: vec![],
        }))
        .await
    }
}

impl NinjaTask for NoopTask {}
